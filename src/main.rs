#[macro_use] extern crate gfx;
#[macro_use] extern crate lazy_static;

use gfx::texture::SamplerInfo;
use gfx::texture::FilterMethod;
use gfx::texture::WrapMode;
use gfx_device_gl::Resources;
use gfx::handle::ShaderResourceView;
use image::GenericImageView;
use std::path::Path;
use gfx::Factory as _;
use gfx_device_gl::Factory;
use std::sync::mpsc::Receiver;
use gfx::Slice;
use image::GenericImage;

////////////////////////////////////////////////////////////////////////////////

mod lane;
mod notes;
mod utils;

fn main() {
    lane::yeah();
    return;
}
