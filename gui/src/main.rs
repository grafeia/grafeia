use grafeia_app::app::App;
use std::alloc::System;

#[global_allocator]
pub static mut THE_ALLOC: System = System;

fn main() {
    env_logger::init();
    let app = App::load().unwrap_or_else(App::build);
    let (data, ext) = app.export();
    std::fs::write(&format!("document.{}", ext), data);

    pathfinder_view::show(app, pathfinder_view::Config {
        zoom: true,
        pan: true
    });
}
