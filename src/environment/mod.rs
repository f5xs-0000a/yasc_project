pub mod actor_wrapper;
pub mod key_bindings;
pub mod state;
pub mod update_routine;

////////////////////////////////////////////////////////////////////////////////

use self::state::GameState;
use crate::{
    environment::{
        actor_wrapper::{
            ActorWrapper as _,
            RenderDetails as _,
            RenderPayload,
            UpdatePayload,
            WrappedAddr,
        },
        update_routine::UpdateEnvelope,
    },
    song_player::song_timer::SongTime,
};
use futures::{
    future::Future as _,
    stream::Stream,
    sync::mpsc::{
        UnboundedReceiver,
        UnboundedSender,
    },
};
use gfx::{
    format::{
        DepthStencil,
        Srgba8,
    },
    handle::{
        DepthStencilView,
        RenderTargetView,
    },
};
use gfx_device_gl::{
    CommandBuffer,
    Factory,
    Resources,
};
use gfx_graphics::{
    Gfx2d,
    TextureContext,
};
use glutin_window::GlutinWindow;
use piston_window::{
    Events,
    Input,
    PistonWindow,
};
use shader_version::glsl::GLSL;
use std::time::Instant;
use tokio_threadpool::ThreadPool;

////////////////////////////////////////////////////////////////////////////////

pub struct GamePrelude {
    threadpool: ThreadPool,

    // we just extracted the fields of PistonWindow here and wrap some of them
    window:  GlutinWindow,
    events:  Events,
    tex_ctx: TextureContext<Factory, Resources, CommandBuffer>,

    output_color: RenderTargetView<Resources, Srgba8>,
    output_stencil: DepthStencilView<Resources, DepthStencil>,
    g2d: Gfx2d<Resources>,
    shdr_ver: GLSL,

    // the current state of the game, but only the address to the actor
    state: WrappedAddr<GameState>,

    // Note: This is wrapped in Option so that we can take it out so that we
    // can mut iu_rx AND GamePrelude at the same time
    iu_rx: Option<UnboundedReceiver<UpdateEnvelope>>,
    // below is meant to be cloned and sent to the game state
    iu_tx: UnboundedSender<UpdateEnvelope>,
}

impl GamePrelude {
    pub fn new() -> GamePrelude {
        use piston_window::WindowSettings;

        // create the threadpool
        let threadpool = ThreadPool::new();

        // declare which version of opengl to use
        //let opengl = piston_window::OpenGL::V3_3;

        // we'll be changing the samples, and vsync soon using settings
        // declare the window
        let pistonwindow: PistonWindow =
            WindowSettings::new("YASC Project", [360, 360])
                .srgb(true)
                .samples(4)
                .vsync(true)
                .build()
                .expect("Failed to create Piston window");

        let output_color = pistonwindow.output_color;
        let output_stencil = pistonwindow.output_stencil;
        let events = pistonwindow.events;
        let window = pistonwindow.window;
        let g2d = pistonwindow.g2d;
        let shdr_ver = GLSL::V3_30;
        let tex_ctx = TextureContext {
            factory: pistonwindow.factory,
            encoder: pistonwindow.encoder,
        };

        let state = GameState::start()
            .start_actor(Default::default(), threadpool.sender().clone());

        let (iu_tx, iu_rx) = UpdateEnvelope::unbounded();

        GamePrelude {
            threadpool,

            window,
            output_color,
            output_stencil,
            events,
            g2d,
            shdr_ver,
            tex_ctx,

            state,
            iu_tx,
            iu_rx: Some(iu_rx),
        }
    }

    pub fn spin_loop(&mut self) {
        use piston_window::{
            Event as E,
            Loop,
        };

        while let Some(e) = self.events.next(&mut self.window) {
            match e {
                // we already handled this
                E::Loop(Loop::Render(_)) => {
                    self.render_procedure();
                    // normally, this should be unreachable!(),
                },

                // handle the inputs of the game
                // TODO: what does the Option<u32> pertain to? (second element)
                E::Input(i, _) => self.update_procedure(Some(i)),

                // handle update requests by handling the initialization
                // requests
                E::Loop(Loop::Update(_)) => {
                    self.update_procedure(None);
                },

                _ => {},
            }
        }
    }

    fn get_game_time(&self) -> GameTime {
        GameTime {
            instant:   Instant::now(),
            song_time: None,
        }
    }

    fn update_procedure(
        &mut self,
        input: Option<Input>,
    )
    {
        use futures::future::Either::*;

        let payload = UpdatePayload {
            event:     input,
            game_time: self.get_game_time(),
            tx:        self.iu_tx.clone(),
            payload:   (),
        };

        let response_fut = self
            .state
            .send(payload)
            // we map the response to an either so we can properly merge it with
            // the iu_rx stream
            .map(|response| B(response))
            // likewise, map the error too
            .map_err(|cancel| B(cancel));

        // temporarily take iu_rx from its container so we can build
        // UpdateWindowParts
        let mut iu_rx = self.iu_rx.take().unwrap();

        {
            let mut uwp = UpdateWindowParts::from_game_prelude(self);

            // now we wait for either the response or how much there is left in
            // the iu_rx.
            let waitable = iu_rx
                .by_ref()
                .map(|env| A(env))
                .map_err(|_| A(()))
                .select(response_fut.into_stream())
                .wait();

            waitable.for_each(|select| {
                match select {
                    Ok(A(env)) => {
                        env.handle(&mut uwp);
                    },

                    Ok(B(_)) => return,

                    Err(_) => unreachable!(),
                }
            });
        }

        // and we put the iu_rx back, now that we're done using the
        // UpdateWindowParts
        self.iu_rx = Some(iu_rx);
    }

    fn render_procedure(&mut self) {
        let payload = RenderPayload::new(
            (),
            self.get_game_time(),
            self.output_color.clone(),
            self.output_stencil.clone(),
            self.shdr_ver.clone(),
        );

        self.state
            .send(payload)
            .map(|response| {
                response.render(&mut RenderWindowParts::from_game_prelude(self))
            })
            .wait();
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct RenderWindowParts<'a> {
    pub tex_ctx: &'a mut TextureContext<Factory, Resources, CommandBuffer>,
    pub window:  &'a mut GlutinWindow,
    pub g2d:     &'a mut Gfx2d<Resources>,
}

pub struct UpdateWindowParts<'a> {
    pub tex_ctx: &'a mut TextureContext<Factory, Resources, CommandBuffer>,
    pub window:  &'a mut GlutinWindow,
    pub glsl:    GLSL,
}

impl<'a> RenderWindowParts<'a> {
    fn from_game_prelude(gp: &'a mut GamePrelude) -> RenderWindowParts<'a> {
        RenderWindowParts {
            window:  &mut gp.window,
            g2d:     &mut gp.g2d,
            tex_ctx: &mut gp.tex_ctx,
        }
    }
}

impl<'a> UpdateWindowParts<'a> {
    fn from_game_prelude(gp: &'a mut GamePrelude) -> UpdateWindowParts<'a> {
        UpdateWindowParts {
            tex_ctx: &mut gp.tex_ctx,
            window:  &mut gp.window,
            glsl:    gp.shdr_ver.clone(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct GameTime {
    pub instant:   Instant,
    pub song_time: Option<SongTime>,
}
