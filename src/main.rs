#[macro_use]
extern crate gfx;

mod environment;
mod pipelines;
mod utils;

use environment::GamePrelude;

fn main() {
    let mut prelude = GamePrelude::new();
    prelude.spin_loop();
}
