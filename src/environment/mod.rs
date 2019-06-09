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
        Sampler,
    },
    Factory as _,
};
use gfx_device_gl::{
    Factory,
    Resources,
};
use gfx_graphics::Gfx2d;
use glutin_window::GlutinWindow;
use parking_lot::Mutex;
use piston_window::{
    Events,
    GfxEncoder,
    Input,
    PistonWindow,
};
use std::{
    sync::Arc,
    time::Instant,
};
use tokio_threadpool::ThreadPool;
use shader_version::glsl::GLSL;

////////////////////////////////////////////////////////////////////////////////

pub struct GamePrelude {
    threadpool: ThreadPool,

    // we just extracted the fields of PistonWindow here and wrap some of them
    window:  GlutinWindow,
    factory: Factory,
    events:  Events,

    encoder: Arc<Mutex<GfxEncoder>>,
    output_color: RenderTargetView<Resources, Srgba8>,
    output_stencil: DepthStencilView<Resources, DepthStencil>,
    g2d: Gfx2d<Resources>,
    sampler: Sampler<Resources>,
    shdr_ver: GLSL,

    // the current state of the game, but only the address to the actor
    state: WrappedAddr<GameState>,

    iu_rx: UnboundedReceiver<UpdateEnvelope>,
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

        let encoder = Arc::new(Mutex::new(pistonwindow.encoder));
        let output_color = pistonwindow.output_color;
        let output_stencil = pistonwindow.output_stencil;
        let events = pistonwindow.events;
        let mut factory = pistonwindow.factory;
        let window = pistonwindow.window;
        let g2d = pistonwindow.g2d;
        let shdr_ver = GLSL::V3_30;

        let state = GameState::start()
            .start_actor(Default::default(), threadpool.sender().clone());

        let sampler = generate_sampler(&mut factory);
        let (iu_tx, iu_rx) = UpdateEnvelope::unbounded();

        GamePrelude {
            threadpool,

            window,
            encoder,
            output_color,
            output_stencil,
            factory,
            events,
            g2d,
            shdr_ver,

            state,
            sampler,
            iu_tx,
            iu_rx,
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

        let mut uwp = UpdateWindowParts::from_game_prelude(self);

        // now we wait for either the response or how much there is left in the
        // iu_rx.
        let waitable = self
            .iu_rx
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

    fn render_procedure(&mut self) {
        let payload = RenderPayload {
            payload: (),
            time:    self.get_game_time(),
        };

        self.state
            .send(payload)
            .map(|response| {
                response.render(RenderWindowParts::from_game_prelude(self))
            })
            .wait();
    }
}

fn generate_sampler(factory: &mut Factory) -> Sampler<Resources> {
    use gfx::texture::{
        FilterMethod,
        SamplerInfo,
        WrapMode,
    };

    let info = SamplerInfo::new(FilterMethod::Anisotropic(8), WrapMode::Clamp);

    factory.create_sampler(info)
}

////////////////////////////////////////////////////////////////////////////////

pub struct RenderWindowParts<'a> {
    pub factory: &'a mut Factory,
    pub window: &'a mut GlutinWindow,
    pub g2d: &'a mut Gfx2d<Resources>,
    pub output_color: &'a RenderTargetView<Resources, Srgba8>,
    pub output_stencil: &'a DepthStencilView<Resources, DepthStencil>,
    pub encoder: Arc<Mutex<GfxEncoder>>,
    pub shdr_ver: GLSL,
}

pub struct UpdateWindowParts<'a> {
    pub factory: &'a mut Factory,
    pub window:  &'a mut GlutinWindow,
}

impl<'a> RenderWindowParts<'a> {
    fn from_game_prelude(gp: &'a mut GamePrelude) -> RenderWindowParts<'a> {
        RenderWindowParts {
            factory: &mut gp.factory,
            window: &mut gp.window,
            g2d: &mut gp.g2d,
            output_color: &mut gp.output_color,
            output_stencil: &mut gp.output_stencil,
            encoder: gp.encoder.clone(),
            shdr_ver: gp.shdr_ver.clone(),
        }
    }
}

impl<'a> UpdateWindowParts<'a> {
    fn from_game_prelude(gp: &'a mut GamePrelude) -> UpdateWindowParts<'a> {
        UpdateWindowParts {
            factory: &mut gp.factory,
            window:  &mut gp.window,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct GameTime {
    pub instant:   Instant,
    pub song_time: Option<SongTime>,
}
