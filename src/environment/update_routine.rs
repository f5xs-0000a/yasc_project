use futures::sync::{
    mpsc::{
        unbounded,
        UnboundedReceiver,
        UnboundedSender,
    },
    oneshot::{
        channel as oneshot,
        Receiver as OneshotReceiver,
        Sender as OneshotSender,
    },
};
use gfx_device_gl::Factory;
use glutin_window::GlutinWindow;

////////////////////////////////////////////////////////////////////////////////

// analogue to EnvelopeInnerTrait
trait UpdateEnvelopeInnerTrait: Send {
    fn handle<'a>(
        &mut self,
        factory: &mut UnsendWindowParts<'a>,
    );
}

////////////////////////////////////////////////////////////////////////////////

// analogue to EnvelopeInner
struct UpdateEnvelopeInner<M>(Option<(OneshotSender<M::Response>, M)>)
where M: CanBeWindowHandled;

impl<M> UpdateEnvelopeInner<M>
where M: CanBeWindowHandled
{
    fn boxed_new(
        msg: M
    ) -> (Box<UpdateEnvelopeInner<M>>, OneshotReceiver<M::Response>) {
        let (tx, rx) = oneshot();
        let env = Box::new(UpdateEnvelopeInner(Some((tx, msg))));

        (env, rx)
    }
}

impl<M> UpdateEnvelopeInnerTrait for UpdateEnvelopeInner<M>
where M: CanBeWindowHandled + Send
{
    fn handle<'a>(
        &mut self,
        wh: &'a mut UnsendWindowParts,
    )
    {
        if let Some((tx, msg)) = self.0.take() {
            let response = WindowHandles::handle(wh, msg);
            tx.send(response);
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct UpdateEnvelope(Box<dyn UpdateEnvelopeInnerTrait>);

impl UpdateEnvelope {
    pub fn new<M>(msg: M) -> (UpdateEnvelope, OneshotReceiver<M::Response>)
    where M: 'static + Send + CanBeWindowHandled {
        let (env, rx) = UpdateEnvelopeInner::boxed_new(msg);

        (UpdateEnvelope(env), rx)
    }

    pub fn handle<'a>(
        mut self,
        uwp: &'a mut UnsendWindowParts,
    )
    {
        self.0.handle(uwp);
    }

    pub fn unbounded(
    ) -> (UnboundedSender<UpdateEnvelope>, UnboundedReceiver<UpdateEnvelope>)
    {
        unbounded()
    }
}

////////////////////////////////////////////////////////////////////////////////

// many implementor, no analogue
pub trait CanBeWindowHandled: 'static + Sized + Send {
    type Response: Send;

    fn handle<'a>(
        self,
        uwp: &mut UnsendWindowParts<'a>,
    ) -> Self::Response;

    fn wrap(self) -> (UpdateEnvelope, OneshotReceiver<Self::Response>) {
        UpdateEnvelope::new(self)
    }
}

////////////////////////////////////////////////////////////////////////////////

// equivalent to handles, only one implementor, and it blanket-implements
trait WindowHandles<T>
where T: CanBeWindowHandled + Send {
    fn handle(
        &mut self,
        t: T,
    ) -> T::Response;
}

////////////////////////////////////////////////////////////////////////////////

// equivalent to an actor
pub struct UnsendWindowParts<'a> {
    pub factory: &'a mut Factory,
    pub window:  &'a mut GlutinWindow,
}

impl<'a, T> WindowHandles<T> for UnsendWindowParts<'a>
where T: CanBeWindowHandled + Send
{
    fn handle(
        &mut self,
        t: T,
    ) -> T::Response
    {
        t.handle(self)
    }
}
