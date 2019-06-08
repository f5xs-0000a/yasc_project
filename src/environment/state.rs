use crate::environment::{
    actor_wrapper::{
        ActorWrapper,
        ContextWrapper,
        RenderDetails,
        RenderPayload,
        RenderableActorWrapper,
        UpdatePayload,
    },
    key_bindings::{
        BindRoles,
        ComposedKeystroke,
    },
};
use bidir_map::BidirMap;
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
    Factory,
    Resources,
};
use gfx_graphics::Gfx2d;
use glutin_window::GlutinWindow;
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
        GameStateRenderDetails {}
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct GameStateRenderDetails {}

impl RenderDetails for GameStateRenderDetails {
    fn render(
        self,
        factory: &mut Factory,
        window: &mut GlutinWindow,
        g2d: &mut Gfx2d<Resources>,
        output_color: &RenderTargetView<Resources, Srgba8>,
        output_stencil: &DepthStencilView<Resources, DepthStencil>,
    )
    {
        // do nothing for now
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum StateEnum {
    TitleScreen,
    Settings,
    Song(()), //LaneGovernor),
}
