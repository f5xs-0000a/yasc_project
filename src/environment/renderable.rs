use core::any::Any;
use futures::sync::oneshot::{
    channel as oneshot_channel,
    Receiver as OneshotReceiver,
    Sender as OneshotSender,
};
use futures::future::Future;
use crate::utils::block_fn;
use futures::sync::mpsc::{
    UnboundedSender,
    UnboundedReceiver,
};
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

/*
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
*/

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

pub struct UpdateReceiver(UnboundedReceiver<UpdateEnvelope>);

pub struct UpdateEnvelope(Box<dyn UpdateEnvelopeInnerTrait>);

// analogue to EnvelopeInnerTrait
trait UpdateEnvelopeInnerTrait {
    fn handle<'a>(&mut self, factory: &mut UnsendWindowParts<'a>);
}

// analogue to EnvelopeInner
struct UpdateEnvelopeInner<M>
where
    M: CanBeWindowHandled,
{
    tx: Option<OneshotSender<M::Response>>,
    msg: Option<M>,
}

impl<M> UpdateEnvelopeInner<M>
where
    M: CanBeWindowHandled,
{
    fn boxed_new(msg: M, tx: OneshotSender<M::Response>) -> Box<UpdateEnvelopeInner<M>> {
        Box::new(UpdateEnvelopeInner {
            tx: Some(tx),
            msg: Some(msg),
        })
    }
}

impl<M> UpdateEnvelopeInnerTrait for UpdateEnvelopeInner<M>
where
    M: CanBeWindowHandled + Send,
{
    //type A = A;

    fn handle<'a>(&mut self, wh: &mut UnsendWindowParts<'a>) {
        /*
        if let Some(msg) = self.msg.take() {
            // let the actor handle the message
            let response = actor.handle(msg, ctx);

            // if we have a sender, we send the message
            if let Some(tx) = self.tx.take() {
                // don't care if it fails
                tx.send(response);
            }
        }
        */
    }
}

// equivalent to an actor
pub struct UnsendWindowParts<'a> {
    factory: &'a mut Factory,
    window: &'a mut GlutinWindow,
}

// equivalent to handles, only one implementor, and it blanket-implements
trait WindowHandles<T>
where T: Send {
}

impl<'a, T> WindowHandles<T> for UnsendWindowParts<'a>
where T: CanBeWindowHandled + Send {
}

// many implementor, no analogue
trait CanBeWindowHandled {
    type Response: Send;

    fn handle<'a>(self, uwp: &mut UnsendWindowParts<'a>) -> Self::Response;
}

/*
---- ATTEMPT 2019-06-05 ----

pub trait InitRequestOutput: Send {
}

impl<T> InitRequestOutput for T
where T: Send {
}

pub trait InitRequest: Send {
    type Output: InitRequestOutput;

    fn init(self, factory: &mut Factory) -> Self::Output;
}

pub struct InitRequestPayload<IR: ?Sized >
where IR: InitRequest, {
    tx: Option<OneshotSender<IR::Output>>,
    func: Box<Fn(&IR, &mut Factory) -> IR::Output + Send>,
    iu: IR,
}

impl<IR: ?Sized> InitRequestPayload<IR>
where IR: InitRequest{
    pub fn init_then_send(
        &mut self,
        factory: &mut Factory,
    )
    {
        match self.tx.take() {
            Some(tx) => { tx.send((self.func)(&self.iu, factory)); },
            None => {},
        }
    }
}

pub type GenericInitRequest = Box<InitRequestPayload<dyn InitRequest<Output = dyn InitRequestOutput>>>;

impl<IR> InitRequestPayload<IR>
where IR: 'static + InitRequest,
      IR::Output: 'static {
    pub fn new_with_receiver<F>(
        func: F,
        iu: IR,
    ) -> (Box<InitRequestPayload<dyn InitRequest<Output = dyn InitRequestOutput>>>,
          OneshotReceiver<IR::Output>)
    //) -> (Box<InitRequestPayload<IR>>, OneshotReceiver<IR::Output>)
    where
        F: 'static + Send + Fn(&IR, &mut Factory) -> IR::Output
    {
        let (tx, rx) = oneshot_channel();

        let ir = InitRequestPayload {
            func: Box::new(func),
            iu,
            tx: Some(tx),
        };

        (Box::new(ir), rx)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub fn unblocking_request_initialization<IR, F>(
    func: F,
    iu: IR,
    iu_tx: &mut UnboundedSender<GenericInitRequest>,
) -> Box<IR::Output>
where
    F: 'static + Send + Fn(&IR, &mut Factory) -> IR::Output,
    IR: 'static + InitRequest,
    IR::Output: ?Sized + 'static,
{
    let (ir, rx) = InitRequestPayload::new_with_receiver(func, iu);
    something(iu_tx, ir);

    Box::new(block_fn(|| rx.wait()).unwrap())
}

fn something<IR, F>(
    iu_tx: &mut UnboundedSender<GenericInitRequest>,
    ir: Box<InitRequestPayload<dyn InitRequest<Output = dyn InitRequestOutput>>>,
)
{
    iu_tx.unbounded_send(ir);
}
*/
