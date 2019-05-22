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

pub trait RenderUnit {
    fn render(
        &self,
        factory: &mut Factory,
        g2d: &mut Gfx2d<Resources>,
        output_color: &RenderTargetView<Resources, Srgba8>,
        output_stencil: &DepthStencilView<Resources, DepthStencil>,
    );
}

pub trait InitializationUnit {
    type Output: InitializationUnitOutput;

    fn initialize(
        self,
        factory: &mut Factory,
    ) -> Self::Output;
}

pub trait InitializationUnitOutput {}

impl<T> InitializationUnitOutput for T {
}

pub trait InitializationRequestTrait<IU>
where IU: InitializationUnit {
    fn initialize_then_send(
        &mut self,
        factory: &mut Factory,
    );
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct InitializationRequest<IU>
where IU: InitializationUnit + Sized {
    tx: Option<OneshotSender<IU::Output>>,
    iu: Option<IU>,
}

impl<IU> InitializationRequest<IU>
where IU: InitializationUnit
{
    pub fn new_with_receiver(
        unit: IU
    ) -> (InitializationRequest<IU>, OneshotReceiver<IU::Output>) {
        let (tx, rx) = oneshot_channel();

        let ir = InitializationRequest {
            iu: Some(unit),
            tx: Some(tx),
        };

        (ir, rx)
    }
}

impl<IU> InitializationRequestTrait<IU> for InitializationRequest<IU>
where IU: InitializationUnit
{
    fn initialize_then_send(
        &mut self,
        factory: &mut Factory,
    )
    {
        let tx = self.tx.take().unwrap();
        let iu = self.iu.take().unwrap();

        tx.send(iu.initialize(factory));
    }
}

pub type GenericInitializationRequest = Box<
    dyn InitializationRequestTrait<
        InitializationUnit<Output = InitializationUnitOutput>,
    >,
>;
