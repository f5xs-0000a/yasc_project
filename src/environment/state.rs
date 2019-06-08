use crate::environment::{
    actor_wrapper::{
        ActorWrapper,
        ContextWrapper,
        HandlesWrapper,
        RenderableActorWrapper,
        UpdatePayload,
    },
    key_bindings::{
        BindRoles,
        ComposedKeystroke,
    },
};
use bidir_map::BidirMap;
use futures::{
    sink::Sink,
    sync::mpsc::Sender,
};
use piston_window::{
    Button,
    Input,
};
use std::{
    collections::VecDeque,
    time::Instant,
};

////////////////////////////////////////////////////////////////////////////////

pub struct GameState {
    keybindings: BidirMap<BindRoles, ComposedKeystroke>,
    state: StateEnum,
    buttons_pressed: Vec<(Button, Instant)>,
}

impl GameState {
    pub fn start() -> GameState {
        GameState {
            // TODO: should be read from a config file
            keybindings: BindRoles::default_keyboard_binding(),
            state: StateEnum::TitleScreen,
            buttons_pressed: Vec::with_capacity(8),
        }
    }

    /*
    pub fn handle_input(
        &mut self,
    )
    {
        use piston_window::{
            keyboard::Key as K,
            Button as B,
            ButtonState,
        };
        use StateEnum as SE;

        // FIXME: honestly, I'm just waving in the dark in here. if anyone can
        // provide a better algorithm/philosophy for registering buttons, PR.
        // this will be a mess for now

        let game_time = input.game_time;
        let time = input.time;
        let input = input.input;
        //let iu_tx = input.iu_tx; unimplemented

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
                        // TODO: replace this soon.

                        /*
                        // request the main event loop to fulfill the
                        // initialization for the lane governor
                        let governor = request_initialization(
                            LGInitRequest::debug_new(),
                            fulfill_lane_governor_init_request,
                            &mut iu_tx,
                        );

                        self.state = SE::Song(governor);
                        */

                        unimplemented!();
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
    */
}

impl ActorWrapper for GameState {
    type Payload = ();

    fn update(
        &mut self,
        payload: UpdatePayload<Self::Payload>,
        ctx: &ContextWrapper<Self>,
    )
    {
        unimplemented!()
    }

    /*
    fn on_message_exhaust(
        &mut self,
        ctx: &ContextWrapper<Self>,
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
    */
}

impl RenderableActorWrapper for GameState {
    type Details = ();
    type Payload = ();

    fn emit_render_details(
        &mut self,
        payload: (),
        ctx: &ContextWrapper<Self>,
    ) -> ()
    {
        unimplemented!()
    }
}

/*
impl Handles<RenderRequest> for GameState {
    type Response = Receiver<()>;

    fn handle(
        &mut self,
        msg: RenderRequest,
        ctx: ContextWrapper<Self>,
    ) -> Self::Response
    {
        let (tx, rx) = channel();
        self.requested_for_render = Some(tx);
        rx
    }
}
*/

/*
impl HandlesWrapper<GameInput> for GameState {
    type Response = ();

    fn handle(
        &mut self,
        msg: GameInput,
        ctx: &ContextWrapper<Self>,
    ) -> Self::Response
    {
        self.pending_inputs.push_front(msg);
    }
}
*/

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum StateEnum {
    TitleScreen,
    Settings,
    Song(()), //LaneGovernor),
}

////////////////////////////////////////////////////////////////////////////////

pub enum RenderState {
    TitleScreen,
    Settings,
    Song,
}
