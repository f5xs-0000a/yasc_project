#[macro_use]
extern crate gfx;
#[macro_use]
extern crate lazy_static;

use gfx::{
    handle::ShaderResourceView,
    texture::{
        FilterMethod,
        SamplerInfo,
        WrapMode,
    },
    Factory as _,
    Slice,
};
use gfx_device_gl::{
    Factory,
    Resources,
};
use image::{
    GenericImage,
    GenericImageView,
};
use std::{
    path::Path,
    sync::mpsc::Receiver,
};

////////////////////////////////////////////////////////////////////////////////

mod lane;
mod notes;
mod utils;

fn main() {
    lane::yeah();
    return;
}
