use grafeia_app::app::App;
use grafeia_core::{State, SiteId};
use std::fs::File;

fn main() {
    env_logger::init();
    let file = File::open(std::env::args().nth(1).expect("no file given")).expect("can't read file");
    let state = State::load(file).unwrap();
    let app = App::from_state(state, SiteId(1));

    pathfinder_view::show(app, pathfinder_view::Config {
        zoom: true,
        pan: true
    });
}
