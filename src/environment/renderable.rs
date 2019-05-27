use core::any::Any;
use futures::sync::oneshot::{
    channel as oneshot_channel,
    Receiver as OneshotReceiver,
    Sender as OneshotSender,
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

////////////////////////////////////////////////////////////////////////////////

pub trait RenderRequest: Send + Sync {
    fn render(
        self,
        factory: &mut Factory,
        window: &mut GlutinWindow;
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
) where F: Fn(Box<I>, &mut Factory) -> Box<O> + Send + Sync + 'static {
    let (ir, rx) = GenericInitRequest(
        Box::new(InitRequest::new_with_receiver(iu, func))
    );
    iu_tx.send(ir);

    block_fn(|| rx.wait())
}
