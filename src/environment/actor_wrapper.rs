use crate::environment::{
    update_routine::UpdateEnvelope,
    GameTime,
};
use futures::sync::mpsc::UnboundedSender;
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
use piston_window::Input;
use sekibanki::{
    Actor,
    ActorBuilder,
    Addr,
    ContextImmutHalf,
    Handles,
    Sender as TPSender,
};

////////////////////////////////////////////////////////////////////////////////

/// A trait for types that can be an actor and may need to use the Factory for
/// some of the routines that require the use of Factory
pub trait ActorWrapper: Send + Sync + Sized {
    type Payload: Send + Sync;

    fn start_actor(
        self,
        builder: ActorBuilder,
        pool: TPSender,
    ) -> WrappedAddr<Self>
    {
        WrappedActor(self).start_actor(builder, pool)
    }

    fn on_start(
        &mut self,
        _ctx: &ContextWrapper<Self>,
    )
    {
        // do nothing by default
    }

    fn on_message_exhaust(
        &mut self,
        _ctx: &ContextWrapper<Self>,
    )
    {
        // do nothing by default
    }

    fn update(
        &mut self,
        payload: UpdatePayload<Self::Payload>,
        ctx: &ContextWrapper<Self>,
    );
}

////////////////////////////////////////////////////////////////////////////////

pub trait HandlesWrapper<T>: ActorWrapper
where T: Send + Sync {
    type Response: Send + Sync;

    fn handle(
        &mut self,
        msg: T,
        ctx: &ContextWrapper<Self>,
    ) -> Self::Response;
}

////////////////////////////////////////////////////////////////////////////////

pub trait RenderableActorWrapper: ActorWrapper {
    type Payload: Send + Sync;
    type Details: RenderDetails;

    fn emit_render_details(
        &mut self,
        payload: RenderPayload<<Self as RenderableActorWrapper>::Payload>,
        ctx: &ContextWrapper<Self>,
    ) -> Self::Details;
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct UpdatePayload<P>
where P: Send + Sync {
    pub event:     Option<Input>,
    pub tx:        UnboundedSender<UpdateEnvelope>,
    pub game_time: GameTime,
    pub payload:   P,
}

impl<P> UpdatePayload<P>
where P: Send + Sync
{
    pub fn another<P2>(
        &self,
        payload: P2,
    ) -> UpdatePayload<P2>
    where
        P2: Send + Sync,
    {
        UpdatePayload {
            event: self.event.clone(),
            tx: self.tx.clone(),
            game_time: self.game_time.clone(),
            payload,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct WrappedActor<A>(pub A)
where A: ActorWrapper + 'static;

impl<A> Actor for WrappedActor<A>
where A: ActorWrapper + 'static
{
    fn on_start(
        &mut self,
        ctx: &ContextImmutHalf<Self>,
    )
    {
        self.0.on_start(ctx);
    }

    /*
    fn on_message_exhaust(
        &mut self,
        ctx: &ContextImmutHalf<Self>,
    )
    {
        self.0.on_message_exhaust(ctx);
    }
    */
}

impl<A> HandlesWrapper<UpdatePayload<A::Payload>> for A
where A: ActorWrapper + 'static
{
    type Response = ();

    fn handle(
        &mut self,
        msg: UpdatePayload<A::Payload>,
        ctx: &ContextWrapper<Self>,
    ) -> Self::Response
    {
        self.update(msg, ctx);
    }
}

impl<A, T> Handles<T> for WrappedActor<A>
where
    A: HandlesWrapper<T>,
    T: Send + Sync,
{
    type Response = A::Response;

    fn handle(
        &mut self,
        msg: T,
        ctx: &ContextImmutHalf<Self>,
    ) -> Self::Response
    {
        self.0.handle(msg, ctx)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub type WrappedAddr<A: 'static + ActorWrapper> = Addr<WrappedActor<A>>;

pub type ContextWrapper<T: 'static + ActorWrapper> =
    ContextImmutHalf<WrappedActor<T>>;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct RenderPayload<P>
where P: Send + Sync {
    pub payload: P,
    pub time:    GameTime,
}

impl<A> HandlesWrapper<RenderPayload<<A as RenderableActorWrapper>::Payload>>
    for A
where A: RenderableActorWrapper
{
    type Response = A::Details;

    fn handle(
        &mut self,
        msg: RenderPayload<<A as RenderableActorWrapper>::Payload>,
        ctx: &ContextWrapper<Self>,
    ) -> Self::Response
    {
        self.emit_render_details(msg, ctx)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct RenderWindowParts<'a> {
    factory: &'a mut Factory,
    window: &'a mut GlutinWindow,
    g2d: &'a mut Gfx2d<Resources>,
    output_color: &'a RenderTargetView<Resources, Srgba8>,
    output_stencil: &'a DepthStencilView<Resources, DepthStencil>,
}

////////////////////////////////////////////////////////////////////////////////

pub trait RenderDetails: Send + Sync {
    fn render<'a>(
        self,
        rwp: RenderWindowParts<'a>,
    );
}
