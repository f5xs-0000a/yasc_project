pub mod key_bindings;
pub mod pipelines;
pub mod state;

////////////////////////////////////////////////////////////////////////////////

use piston_window::Input;
use piston_window::PistonWindow;
use gfx::GfxEncoder;
use tokio_threadpool::ThreadPool;
use std::sync::Arc;
use parking_lot::Mutex;

////////////////////////////////////////////////////////////////////////////////

pub struct GamePrelude {
    threadpool: ThreadPool,

    // we just extracted the fields of PistonWindow here and wrap some of them
    window: Arc<Mutex<PistonWindow>>,
    encoder: Arc<Mutex<GfxEncoder>>,
    //device: Device,
    output_color: RenderTargetView<Resources, Srgba8>,
    output_stencil: DepthStencilView<Resources, DepthStencil>,
    //g2d: Gfx2d<Resources>,
    // I don't know if we should wrap factory but we did anyway
    factory: Arc<Mutex<Factory>>,
    // we are not going to wrap events since it is not going to be passed around
    // and only GamePrelude will have the direct access to events
    events: Events,

    state: Addr<GameState>,
    render_helper: Addr<RenderHelper>,
}

impl Game {
    pub fn new() -> GamePrelude {
        // create the threadpool
        let threadpool = ThreadPool::new();

        // declare which version of opengl to use
        let opengl = OpenGL::V3_3;

        // we'll be changing the samples, and vsync soon using settings
        // declare the window
        let mut pistonwindow = WindowSettings::new("YASC Project", [360, 360])
            .opengl(opengl)
            .srgb(true)
            .samples(4)
            .vsync(true)
            .build()
            .expect("Failed to create Piston window");
        // Nobody: 
        // F5XS: for the love of god, do not enable benchmark mode for the
        // render loop. the event loop will pump out render events faster than
        // the render helper could provided responses

        let encoder = Arc::new(Mutex::new(pistonwindow.encoder));
        let output_color = pistonwindow.output_window;
        let output_stencil = pistonwindow.output_stencil;
        let events = pistonwindow.events;
        let factory = Arc::new(Mutex::new(pistonwindow.factory));
        let window = Arc::new(Mutex::new(pistonwindow.window));

        let render_helper = RenderHelper::new()
            .start_actor(threadpool.sender());
        let state = GameState::start().start_actor(threadpool.sender());

        GamePrelude {
            threadpool,

            window,
            encoder,
            output_color,
            output_stencil,
            factory,
            events,
    
            state,
            render_helper,
        }
    }

    pub fn loop(&mut self) {
        while let Some(e) = self.window.next() {

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

    fn handle_inputs(&mut self, input: Input) {
        let timed = GameInput {
            input,
            time: self.get_game_time(),
        };

        let response = self.state.send(timed);
    }

    fn render_procedure(&self) {
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
        let response = self.state.send(request);

        let forward = RenderHelperForwardMsg {
            factory: self.factory.clone(),
            window: self.window.clone(),
            encoder: self.encoder.clone(),
            response,
        };
        self.render_helper.send(forward);
    }
}

impl Actor for GamePrelude {
}

////////////////////////////////////////////////////////////////////////////////

/// A message sent by the game prelude to the game state, asking the state to
/// produce its render state
pub struct RenderRequest {
    pub factory: Arc<Mutex<Factory>>,
    pub output_color: RenderTargetView<Resources, Srgba8>,
    pub output_stencil: DepthStencilView<Resources, DepthStencil>,

    // we're going to implement time soon
    pub time: (),
}

////////////////////////////////////////////////////////////////////////////////

/// A message sent by the game prelude to the render helper, asking the helper
/// to render the frame given the response by the game state
pub struct RenderHelperForwardMsg {
    pub factory: Arc<Mutex<Factory>>,
    pub window: Arc<Mutex<PistonWindow>>,
    pub encoder: Arc<Mutex<GfxEncoder>>,

    pub response: ResponseFuture<GameState, RenderRequest>,
}

////////////////////////////////////////////////////////////////////////////////

/// A messag sent by the game prelude to the game state, telling that an input
/// has been made, accompanied by the time when it was input, if available.
pub struct GameInput {
    input: Input,
    time: (),
}
