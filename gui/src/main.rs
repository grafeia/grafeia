#[macro_use] extern crate log;

mod app;
mod view;

use app::App;
use view::Interactive;

fn main() {
    env_logger::init();
    view::show(App::load().unwrap_or_else(App::build));
}
