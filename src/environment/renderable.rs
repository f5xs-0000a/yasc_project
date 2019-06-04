use core::any::Any;
use futures::sync::oneshot::{
    channel as oneshot_channel,
    Receiver as OneshotReceiver,
    Sender as OneshotSender,
};

use crate::utils::block_fn;
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
use piston_window::Event;
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
    //type

    fn update(
        &mut self,
        payload: UpdatePayload<Self::Payload>,
        ctx: &ContextImmutHalf<WrappedActor<Self>>,
    );

    fn start_actor(
        self,
        builder: ActorBuilder,
        pool: TPSender,
    ) -> Addr<WrappedActor<Self>>
    {
        WrappedActor(self).start_actor(builder, pool)
    }

    fn on_start(
        &mut self,
        ctx: &ContextImmutHalf<WrappedActor<Self>>,
    )
    {
        // do nothing by default
    }

    fn on_message_exhaust(
        &mut self,
        ctx: &ContextImmutHalf<WrappedActor<Self>>,
    )
    {
        // do nothing by default
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait HandlesWrapper<T>: ActorWrapper
where T: Send + Sync {
    type Response: Send + Sync;

    fn handle(
        &mut self,
        msg: T,
        ctx: &ContextImmutHalf<WrappedActor<Self>>,
    ) -> Self::Response;
}

////////////////////////////////////////////////////////////////////////////////

pub trait RenderableActorWrapper: ActorWrapper {
    type Payload: Send + Sync;
    type Details: RenderDetails;

    fn emit_render_details(
        &mut self,
        payload: RenderPayload<<Self as RenderableActorWrapper>::Payload>,
        ctx: &ContextImmutHalf<WrappedActor<Self>>,
    ) -> Self::Details;
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct UpdatePayload<P>
where P: Send + Sync {
    event:   Option<Event>,
    tx:      (), // unimplemented
    payload: P,
}

impl<P> UpdatePayload<P>
where P: Send + Sync
{
    pub fn new(
        event: Option<Event>,
        tx: (),
        payload: P,
    ) -> UpdatePayload<P>
    {
        UpdatePayload {
            event,
            tx,
            payload,
        }
    }

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

    fn on_message_exhaust(
        &mut self,
        ctx: &ContextImmutHalf<Self>,
    )
    {
        self.0.on_message_exhaust(ctx);
    }
}

impl<A> HandlesWrapper<UpdatePayload<A::Payload>> for A
where A: ActorWrapper + 'static
{
    type Response = ();

    fn handle(
        &mut self,
        msg: UpdatePayload<A::Payload>,
        ctx: &ContextImmutHalf<WrappedActor<Self>>,
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

#[derive(Debug, Clone)]
pub struct RenderPayload<P>
where P: Send + Sync {
    tx:      (), // unimplemented
    payload: P,
}

impl<A> HandlesWrapper<RenderPayload<<A as RenderableActorWrapper>::Payload>>
    for A
where A: RenderableActorWrapper
{
    type Response = A::Details;

    fn handle(
        &mut self,
        msg: RenderPayload<<A as RenderableActorWrapper>::Payload>,
        ctx: &ContextImmutHalf<WrappedActor<Self>>,
    ) -> Self::Response
    {
        self.emit_render_details(msg, ctx)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait RenderDetails: Send + Sync {
    fn render(
        self,
        factory: &mut Factory,
        window: &mut GlutinWindow,
        g2d: &mut Gfx2d<Resources>,
        output_color: &RenderTargetView<Resources, Srgba8>,
        output_stencil: &DepthStencilView<Resources, DepthStencil>,
    );
}

////////////////////////////////////////////////////////////////////////////////

pub enum InitRequest<I, O>
where
    I: ?Sized,
    O: ?Sized, {
    Pending {
        iu:   Box<I>,
        tx:   OneshotSender<Box<O>>,
        func: Box<Fn(Box<I>, &mut Factory) -> Box<O> + Send + Sync>,
    },
    Fulfilled,
}

impl<I, O> InitRequest<I, O> {
    pub fn new_with_receiver<F>(
        func: F,
        iu: I,
    ) -> (InitRequest<I, O>, OneshotReceiver<Box<O>>)
    where
        F: Fn(Box<I>, &mut Factory) -> Box<O> + Send + Sync + 'static,
    {
        let (tx, rx) = oneshot_channel();

        let ir = InitRequest::Pending {
            func: Box::new(func),
            iu: Box::new(iu),
            tx,
        };

        (ir, rx)
    }
}

impl<I, O> InitRequest<I, O>
where
    I: ?Sized,
    O: ?Sized,
{
    fn init_then_send(
        &mut self,
        factory: &mut Factory,
    )
    {
        let mut swapped = InitRequest::Fulfilled;
        std::mem::swap(self, &mut swapped);

        match swapped {
            InitRequest::Pending {
                iu,
                tx,
                func,
            } => {
                tx.send((func)(iu, factory));
            },

            _ => {
                // TODO: log unreachable in here.
            },
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct GenericInitRequest(
    Box<InitRequest<dyn Send + Sync, dyn Send + Sync>>,
);

impl GenericInitRequest {
    pub fn init_then_send(
        &mut self,
        factory: &mut Factory,
    )
    {
        self.0.init_then_send(factory);
    }
}

////////////////////////////////////////////////////////////////////////////////

pub fn request_initialization<I, O, F>(
    func: F,
    iu: I,
    &mut iu_tx: UnboundedSender<GenericInitRequest>,
) where
    F: Fn(Box<I>, &mut Factory) -> Box<O> + Send + Sync + 'static,
{
    let (ir, rx) =
        GenericInitRequest(Box::new(InitRequest::new_with_receiver(iu, func)));
    iu_tx.send(ir);

    block_fn(|| rx.wait())
}
