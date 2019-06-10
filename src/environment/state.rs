use crate::{
    environment::{
        actor_wrapper::{
            ActorWrapper,
            ContextWrapper,
            RenderDetails,
            RenderPayload,
            RenderResponseFuture,
            RenderableActorWrapper,
            UpdatePayload,
            WrappedAddr,
        },
        key_bindings::{
            BindRoles,
            ComposedKeystroke,
        },
        RenderWindowParts,
    },
    song_player::governor::LaneGovernor,
};
use bidir_map::BidirMap;
use futures::future::Future as _;
use piston_window::{
    Button,
    Input,
};
use std::time::Instant;

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
}

impl ActorWrapper for GameState {
    type Payload = ();

    fn update(
        &mut self,
        payload: UpdatePayload<Self::Payload>,
        _: &ContextWrapper<Self>,
    )
    {
        use self::StateEnum::*;
        use piston_window::ButtonState;

        // update the buttons_pressed
        let mut new_press = None;
        if let &Some(Input::Button(b)) = &payload.event {
            new_press = Some(b.button.clone());

            if b.state == ButtonState::Press {
                self.buttons_pressed.push((
                    b.button.clone(),
                    payload.game_time.instant.clone(),
                ));
            }
            else {
                self.buttons_pressed.retain(|x| x.0 != b.button);
            }
        }

        match &mut self.state {
            Song(lg_addr) => {}, // unimplemented
            _ => {},
        }
    }
}

impl RenderableActorWrapper for GameState {
    type Details = GameStateRenderDetails;
    type Payload = ();

    fn emit_render_details(
        &mut self,
        payload: RenderPayload<()>,
        _: &ContextWrapper<Self>,
    ) -> Self::Details
    {
        use self::{
            GameStateRenderDetails as GSRD,
            StateEnum as SE,
        };

        match &mut self.state {
            SE::Uninitialized => GSRD::Uninitialized,
            SE::TitleScreen => GSRD::TitleScreen,
            SE::Settings => GSRD::Settings,
            SE::SongSelection => GSRD::SongSelection,
            SE::Song(ref mut addr) => GSRD::Song(addr.send(payload)),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub enum GameStateRenderDetails {
    Uninitialized,

    TitleScreen,

    Settings,

    SongSelection,

    Song(RenderResponseFuture<LaneGovernor>),
}

impl RenderDetails for GameStateRenderDetails {
    fn render<'a>(
        self,
        rwp: &mut RenderWindowParts<'a>,
    )
    {
        use self::GameStateRenderDetails::*;

        match self {
            Song(mut response) => {
                match response.wait() {
                    Ok(r) => r.render(rwp),
                    _ => {}, /* response is cancelled, so we just don't
                              * render
                              * TODO: we might want to match that and log the
                              * result */
                }
            },
            _ => {}, // unimplemented
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub enum StateEnum {
    Uninitialized,
    TitleScreen,
    Settings,
    SongSelection,
    Song(WrappedAddr<LaneGovernor>),
}
