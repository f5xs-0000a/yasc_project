use crate::{
    environment::{
        key_bindings::{
            BindRoles,
            ComposedKeystroke,
        },
        GameInput,
        RenderRequest,
    },
    song_player::governor::LaneGovernor,
};
use bidir_map::BidirMap;
use fnv::FnvHashSet as HashSet;
use futures::{
    sink::Sink,
    sync::mpsc::{
        channel,
        Receiver,
        Sender,
    },
};
use piston_window::{
    Button,
    Input,
};
use sekibanki::{
    Actor,
    ContextImmutHalf,
    Handles,
};
use std::{
    collections::{
        HashMap,
        VecDeque,
    },
    time::Instant,
};

////////////////////////////////////////////////////////////////////////////////

pub struct GameState {
    keybindings: BidirMap<BindRoles, ComposedKeystroke>,
    state:       StateEnum,

    requested_for_render: Option<Sender<()>>,
    pending_inputs:       VecDeque<GameInput>,

    buttons_pressed: Vec<(Button, Instant)>,
}

impl GameState {
    pub fn start() -> GameState {
        GameState {
            // TODO: should be read from a config file
            keybindings: BindRoles::default_keyboard_binding(),
            state: StateEnum::TitleScreen,
            requested_for_render: None,
            // 8 is an arbitrary number
            pending_inputs:  VecDeque::with_capacity(8),
            buttons_pressed: Vec::with_capacity(8),
        }
    }

    pub fn handle_input(
        &mut self,
        input: GameInput,
    )
    {
        use piston_window::{
            keyboard::Key as K,
            Button as B,
            ButtonState,
        };
        use GameState as GS;
        use StateEnum as SE;

        // FIXME: honestly, I'm just waving in the dark in here. if anyone can
        // provide a better algorithm/philosophy for registering buttons, PR.
        // this will be a mess for now

        let game_time = input.game_time;
        let time = input.time;
        let input = input.input;

        let mut new_press = None;

        // update the buttons_pressed
        if let &Input::Button(b) = &input {
            new_press = Some(b.button.clone());

            if b.state == ButtonState::Press {
                self.buttons_pressed.push((b.button.clone(), time));
            }
            else {
                self.buttons_pressed.retain(|x| x.0 != b.button);
            }
        }

        match &self.state {
            SE::TitleScreen => {
                if let Some(ref new_press) = &new_press {
                    if *new_press == B::Keyboard(K::Return) {
                        // FIXME: remove this debug-only thingy
                        let governor = LaneGovernor::debug_new();

                        self.state = SE::Song(governor);
                    }
                }
            },

            SE::Song(ref governor) => {
                // nope, nothing here for now
            },

            SE::Settings => {},
        }
    }

    pub fn provide_render_state(&self) -> () {
        // this place will be very messy for now

    }
}

impl Actor for GameState {
    fn on_message_exhaust(
        &mut self,
        ctx: &ContextImmutHalf<Self>,
    )
    {
        use std::mem::swap;

        // Handle all inputs
        let mut empty = VecDeque::with_capacity(self.pending_inputs.capacity());
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

/*
impl Handles<RenderRequest> for GameState {
    type Response = Receiver<()>;

    fn handle(
        &mut self,
        msg: RenderRequest,
        ctx: ContextImmutHalf<Self>,
    ) -> Self::Response
    {
        let (tx, rx) = channel();
        self.requested_for_render = Some(tx);
        rx
    }
}
*/

impl Handles<GameInput> for GameState {
    type Response = ();

    fn handle(
        &mut self,
        msg: GameInput,
        ctx: &ContextImmutHalf<Self>,
    ) -> Self::Response
    {
        self.pending_inputs.push_front(msg);
    }
}

////////////////////////////////////////////////////////////////////////////////

pub enum StateEnum {
    TitleScreen,
    Settings,
    Song(LaneGovernor),
}

////////////////////////////////////////////////////////////////////////////////

pub enum RenderState {
    TitleScreen,
    Settings,
    Song,
}
