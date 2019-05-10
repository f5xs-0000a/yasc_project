use sekibanki::Handles;
use sekibanki::Actor;
use sekibanki::ContextImmutHalf;
use crate::environment::RenderHelperForwardMsg;

////////////////////////////////////////////////////////////////////////////////

pub struct RenderHelper {
    // unimplemented
}

impl Actor for RenderHelper {
}

impl Handles<RenderHelperForwardMsg> for RenderHelper {
    type Response = ();

    fn handle(&mut self, msg: RenderHelperForwardMsg, ctx: &mut ContextImmutHalf<Self>)
    -> Self::Response {
    }
}
