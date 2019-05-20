pub mod key_bindings;
pub mod state;

////////////////////////////////////////////////////////////////////////////////

use gfx::Factory as _;
use self::state::GameState;
use futures::sync::mpsc::{
    Receiver,
    Sender,
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
};
use gfx_device_gl::{
    Factory,
    Resources,
};
use glutin_window::GlutinWindow;
use parking_lot::Mutex;
use piston_window::{
    EventLoop,
    Events,
    GfxEncoder,
    Input,
    PistonWindow,
};
use sekibanki::{
    Actor,
    Addr,
    ResponseFuture,
};
use std::{
    sync::Arc,
    time::Instant,
};
use tokio_threadpool::ThreadPool;

////////////////////////////////////////////////////////////////////////////////

pub struct GamePrelude {
    threadpool: ThreadPool,

    // we just extracted the fields of PistonWindow here and wrap some of them
    window:  GlutinWindow,
    encoder: Arc<Mutex<GfxEncoder>>,
    //device: Device,
    output_color:   RenderTargetView<Resources, Srgba8>,
    output_stencil: DepthStencilView<Resources, DepthStencil>,
    //g2d: Gfx2d<Resources>,
    // I don't know if we should wrap factory but we did anyway
    factory: Factory,
    // since calling Events::next() requires a &mut GlutinWindow, we place this
    // here and rethink our life choices
    events: Events,

    state: Addr<GameState>,

    sampler: Sampler<Resources>,
}

impl GamePrelude {
    pub fn new() -> GamePrelude {
        use piston_window::WindowSettings;

        // create the threadpool
        let threadpool = ThreadPool::new();

        // declare which version of opengl to use
        let opengl = piston_window::OpenGL::V3_3;

        // we'll be changing the samples, and vsync soon using settings
        // declare the window
        let mut pistonwindow: PistonWindow =
            WindowSettings::new("YASC Project", [360, 360])
                .opengl(opengl)
                .srgb(true)
                .samples(4)
                .vsync(true)
                .build()
                .expect("Failed to create Piston window");

        let encoder = Arc::new(Mutex::new(pistonwindow.encoder));
        let output_color = pistonwindow.output_color;
        let output_stencil = pistonwindow.output_stencil;
        let events = pistonwindow.events.lazy(true);
        let mut factory = pistonwindow.factory;
        let window = pistonwindow.window;

        let state = GameState::start()
            .start_actor(Default::default(), threadpool.sender().clone());

        let sampler = generate_sampler(&mut factory);

        GamePrelude {
            threadpool,

            window,
            encoder,
            output_color,
            output_stencil,
            factory,
            events,

            state,
            sampler,
        }
    }

    pub fn spin(&mut self) {
        use piston_window::{
            Event as E,
            Loop,
        };

        while let Some(e) = self.events.next(&mut self.window) {
            // handle the rendering of the game
            match &e {
                E::Loop(Loop::Render(_)) => {
                    self.render_procedure();
                    continue;
                },
                _ => {},
            }

            match e {
                // we already handled this
                E::Loop(Loop::Render(_)) => unreachable!(),

                // handle the inputs of the game
                E::Input(b) => self.handle_inputs(b),

                _ => {},
            } // match
        } // while
    }

    fn get_game_time(&self) -> () {
        ()
    }

    fn handle_inputs(
        &mut self,
        input: Input,
    )
    {
        let timed = GameInput {
            input,
            time: Instant::now(),
            game_time: self.get_game_time(),
        };

        let response = self.state.send(timed);
    }

    /*
    fn pre_render_procedure(&self) {
        // The design philosophy behind the rendering is that we assume each
        // groupable object that requires rendering to be an actor that will
        // need their own independent computer. A message is sent to the actor,
        // telling the actor to calculate is render state. After computing the
        // state, it is sent back to the sender as a reply. The sender, given
        // the render state, renders the frame.
        //
        // This results in the utilization of all cores of the CPU,
        // theoretically yielding a faster rendering time, as compared to only
        // a single core being bottlenecked with tasks.
        //
        // We also do this so that we would not starve the game state
        // from its inputs. We need to have the most minimal delay between
        // inputs and the game state update. After all, this is a rhythm game.
        //
        // The actor system is provided by Sekibanki.
        // (the library, not the rokurokubi)

        // the request for render state is sent to the game state, along
        // with a copy of Factory
        // the response is sent to the render helper, along with a copy
        // of the encoder
        // we do this so that we would not starve the game state from
        // its inputs

        // NOTE: in order to implement this, we have to take all the
        // elements of Piston window to ourselves and wrap them however
        // we want

        let request = RenderRequest {
            factory: self.factory.clone(),
            output_color: self.output_color.clone(),
            output_stencil: self.output_stencil.clone(),
            time: self.get_game_time(),
        };
        let response: () = self.state.send(request);

        self.render_procedure(response);
    }
    */

    fn render_procedure(&self) {
        unimplemented!();
    }
}

fn generate_sampler(factory: &mut Factory) -> Sampler<Resources> {
    use gfx::texture::SamplerInfo;
    use gfx::texture::FilterMethod;
    use gfx::texture::WrapMode;

    let info = SamplerInfo::new(
        FilterMethod::Anisotropic(8),
        WrapMode::Clamp,
    );

    factory.create_sampler(info)
}

////////////////////////////////////////////////////////////////////////////////

/// A message sent by the game prelude to the game state, asking the state to
/// produce its render state
pub struct RenderRequest {
    pub output_color:   RenderTargetView<Resources, Srgba8>,
    pub output_stencil: DepthStencilView<Resources, DepthStencil>,

    // we're going to implement time soon
    pub time: (),
}

////////////////////////////////////////////////////////////////////////////////

/// A message sent by the game prelude to the game state, telling that an input
/// has been made, accompanied by the time when it was input, if available.
pub struct GameInput {
    input:     Input,
    time:      Instant,
    game_time: (),
}
