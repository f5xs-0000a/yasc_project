use bidir_map::BiDirMap;
use self::key_bindings::BindRoles;
use self::key_bindings::ComposedKeystroke;
use piston_window::Input;
use std::time::Instant;
use futures::mpsc::{
    Receiver,
    Sender,
    channel,
};
use fnv::FnvHashSet as HashSet;

////////////////////////////////////////////////////////////////////////////////

pub struct GameState {
    keybindings: BiDirMap<BindRoles, ComposedKeystroke>,
    state: StateEnum,

    // might not contain a bool, but rather a one-shot instead
    requested_for_render: Option<Sender<()>>,
    pending_inputs: VecDeque<GameInput>,

    buttons_pressed: HashSet<Inputs>,
}

impl GameState {
    pub fn start() -> GameState {
        GameState {
            // TODO: should be read from a config file
            keybindings: BindRoles::default_keyboard_binding(),
            state: StateEnum::Title,
            requested_for_render: None,
            // 8 is an arbitrary number
            pending_inputs: VecDeque::with_capacity(8),
            inputs_pressed: HashSet::with_capacity(8),
        }
    }

    pub fn handle_input(&mut self, input: GameInput) {
        // honestly, I'm just waving in the dark in here. if anyone can provide
        // a better algorithm/philosophy for registering buttons, PR.

        let time = input.time;
        let input = input.input;

        let mut new_press = None;

        // update the buttons_pressed
        if let Button(ref b) = &input {
            new_press = Some(b.button.clone());

            if b.state == ButtonState::Press {
                self.buttons_pressed.insert(b.button.clone());
            }

            else {
                self.buttons_pressed.remove(&b.button);
            }
        }

        if let Some(new_button) = new_press {
            let new_press_general =
                GeneralizedKeystroke::from_button(new_button);

            self.keybindings
                .iter()
                .filter(|x| self.contains(new_press_general));
        }
    }

    pub fn provide_render_state(&self) -> () {
        // this place will be very messy for now
    }
}

impl Actor for GameState {
    fn on_message_exhaust(&mut self, ctx: ContextImmutHalf<Self>) {
        use std::mem::swap;

        // Handle all inputs
        let mut empty = VecDeque::with_capacity(pending_inputs.capacity());
        swap(&mut empty, &mut self.pending_inputs);
        for input in empty.drain(..) {
            self.handle_input(input);
        }
        // pending_inputs should be empty at this point, as it should be

        // The philosophy behind this is that we pool all request for renders
        // into one, in case there happened to have a bottleneck of request for
        // renders. That way, we don't make a new render state for each
        // request that has been made, just one for the latest request.
        if let Some(tx) = self.requested_for_render.take() {
            let response = self.provide_render_state();
            tx.send(response);
        }
    }
}

impl Handles<RenderRequest> for GameState {
    type Response = Receiver<()>;

    fn handle(&mut self, msg: RenderRequest, ctx: ContextImmutHalf<Self>)
    -> Self::Response {
        let (tx, rx) = channel();
        self.requested_for_render = Some(tx);
        rx
    }
}

impl Handles<Input> for GameState {
    type Response = ();

    fn handle(&mut self, msg: Input, ctx: ContextImmutHalf<Self>)
    -> Self::Response {
        self.pending_inputs.push(msg);
    }
}

////////////////////////////////////////////////////////////////////////////////

pub enum StateEnum {
    TitleScreen,
    Song,
}

////////////////////////////////////////////////////////////////////////////////
