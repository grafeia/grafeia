#[macro_use] extern crate log;

use grafeia_app::app::App;
use grafeia_core::{State, SiteId};
use std::fs::File;
use std::io::BufReader;

fn main() {
    env_logger::init();
    let file = File::open(std::env::args().nth(1).expect("no file given")).expect("can't read file");

    info!("opening document");
    let state = State::load(BufReader::new(file)).unwrap();
    let app = App::from_state(state, SiteId(1));

    pathfinder_view::show(app, pathfinder_view::Config {
        zoom: true,
        pan: true
    });
}
