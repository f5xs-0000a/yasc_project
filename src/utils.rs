use crate::gfx::Factory as _;
use gfx::{
    format::{
        Srgb,
        Vec4,
        R8_G8_B8_A8,
    },
    handle::{
        RenderTargetView,
        Sampler,
        ShaderResourceView,
        Texture,
    },
};
use gfx_device_gl::{
    CommandBuffer,
    Factory,
    Resources,
};
use gfx_graphics::{
    Filter,
    TextureContext,
    TextureSettings,
};
use image::{
    ImageBuffer,
    Rgba,
};
use num_traits::{
    Float,
    One,
};
use std::ops::{
    Add,
    Div,
    Mul,
    Neg,
    Sub,
};

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct TextureWithTarget {
    pub tex: Texture<Resources, R8_G8_B8_A8>,
    pub srv: ShaderResourceView<Resources, Vec4<f32>>,
    pub rtv: RenderTargetView<Resources, (R8_G8_B8_A8, Srgb)>,

    pub sampler: Sampler<Resources>,
}

impl TextureWithTarget {
    pub fn new(
        w: u16,
        h: u16,
        factory: &mut Factory,
    ) -> TextureWithTarget
    {
        use gfx::texture::{
            FilterMethod,
            SamplerInfo,
            WrapMode,
        };

        let (tex, srv, rtv) = factory.create_render_target(w, h).unwrap();

        let sampler_info =
            SamplerInfo::new(FilterMethod::Bilinear, WrapMode::Clamp);
        let sampler = factory.create_sampler(sampler_info);

        TextureWithTarget {
            tex,
            srv,
            rtv,
            sampler,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub fn linear_map<T>(
    x_i: T,
    x_min: T,
    x_max: T,
    y_min: T,
    y_max: T,
) -> T
where
    T: Copy
        + Add<T, Output = T>
        + Sub<T, Output = T>
        + Div<T, Output = T>
        + Mul<T, Output = T>,
{
    let output = (x_i - x_min) / (x_max - x_min) * (y_max - y_min) + y_min;
    output
}

pub fn sigmoid<T>(x: T) -> T
where T: Float + Neg + Add<T, Output = T> + One {
    (T::one() + (-x).exp()).recip()
}

pub fn block_fn<F, T>(f: F) -> T
where F: FnOnce() -> T {
    use tokio_threadpool::blocking;

    // perform blocking
    let blocker = blocking(f).expect(
        "block_fn() must be called if the calling thread is on a ThreadPool.",
    );

    // extract data
    match blocker {
        futures::Async::Ready(smthng) => smthng,
        _ => {
            panic!(
                "Maximum number of blocking threads reached!. You may want to \
                 consider increasing this."
            )
        },
    }
}
