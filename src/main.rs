#[macro_use]
extern crate gfx;
#[macro_use]
extern crate lazy_static;

////////////////////////////////////////////////////////////////////////////////

mod environment;
mod pipelines;
mod song_player;
mod utils;

////////////////////////////////////////////////////////////////////////////////

fn main() {
    let mut prelude = environment::GamePrelude::new();
    prelude.spin_loop();
}
