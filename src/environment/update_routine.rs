use core::any::Any;
use futures::sync::oneshot::{
    channel as oneshot,
    Receiver as OneshotReceiver,
    Sender as OneshotSender,
};
use futures::future::Future;
use crate::utils::block_fn;
use futures::sync::mpsc::{
    UnboundedSender,
    UnboundedReceiver,
    unbounded,
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

pub struct UpdateReceiver(UnboundedReceiver<UpdateEnvelope>);

impl UpdateReceiver {
    pub fn new() -> (UpdateReceiver, UnboundedSender<UpdateEnvelope>) {
        let (tx, rx) = unbounded();

        (UpdateReceiver(rx), tx)
    }
}

////////////////////////////////////////////////////////////////////////////////

// analogue to EnvelopeInnerTrait
trait UpdateEnvelopeInnerTrait {
    fn handle<'a>(&mut self, factory: &mut UnsendWindowParts<'a>);
}

////////////////////////////////////////////////////////////////////////////////

// analogue to EnvelopeInner
struct UpdateEnvelopeInner<M>(Option<(OneshotSender<M::Response>, M)>)
where
    M: CanBeWindowHandled;
//{
    //tx: Option<OneshotSender<M::Response>>,
    //msg: Option<M>,
//}

impl<M> UpdateEnvelopeInner<M>
where
    M: CanBeWindowHandled,
{
    fn boxed_new(msg: M, tx: OneshotSender<M::Response>) -> Box<UpdateEnvelopeInner<M>> {
        /*
        Box::new(UpdateEnvelopeInner {
            tx: Some(tx),
            msg: Some(msg),
        })
        */
        Box::new(UpdateEnvelopeInner(Some((tx, msg))))
    }
}

impl<M> UpdateEnvelopeInnerTrait for UpdateEnvelopeInner<M>
where
    M: CanBeWindowHandled + Send,
{
    fn handle<'a>(&mut self, wh: &'a mut UnsendWindowParts) {
        if let Some((tx, msg)) = self.0.take() {
            let response = WindowHandles::handle(wh, msg);
            tx.send(response);
        }
        /*
        if let Some(msg) = self.msg.take() {
            // let the actor handle the message
            let response = WindowHandles::handle(wh, msg);

            // if we have a sender, we send the message
            if let Some(tx) = self.tx.take() {
                // don't care if it fails
                tx.send(response);
            }
        }
        */
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct UpdateEnvelope(Box<dyn UpdateEnvelopeInnerTrait>);

impl UpdateEnvelope {
    pub fn new<M>(msg: M, tx: OneshotSender<M::Response>) -> (UpdateEnvelope, OneshotReceiver<M::Response>)
    where
        M: 'static + Send + CanBeWindowHandled,
    {
        let (tx, rx) = oneshot();
        let env = UpdateEnvelope(UpdateEnvelopeInner::boxed_new(msg, tx));

        (env, rx)
    }

    pub fn handle<'a>(mut self, uwp: &'a mut UnsendWindowParts) {
        self.0.handle(uwp);
    }
}

////////////////////////////////////////////////////////////////////////////////

// many implementor, no analogue
pub trait CanBeWindowHandled {
    type Response: Send;

    fn handle<'a>(self, uwp: &mut UnsendWindowParts<'a>) -> Self::Response;
}

////////////////////////////////////////////////////////////////////////////////

// equivalent to handles, only one implementor, and it blanket-implements
trait WindowHandles<T>
where T: CanBeWindowHandled + Send {
    fn handle(&mut self, t: T) -> T::Response;
}

////////////////////////////////////////////////////////////////////////////////

// equivalent to an actor
pub struct UnsendWindowParts<'a> {
    factory: &'a mut Factory,
    window: &'a mut GlutinWindow,
}

impl<'a, T> WindowHandles<T> for UnsendWindowParts<'a>
where T: CanBeWindowHandled + Send {
    fn handle(&mut self, t: T) -> T::Response {
        t.handle(self)
    }
}
