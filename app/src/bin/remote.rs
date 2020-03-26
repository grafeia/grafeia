#[macro_use] extern crate log;

use grafeia_app::net::NetworkApp;
use grafeia_core::{State, SiteId};
use std::fs::File;
use std::io::BufReader;

fn main() {
    env_logger::init();
    let app = NetworkApp::new();

    pathfinder_view::show(app, pathfinder_view::Config {
        zoom: true,
        pan: true
    });
}
