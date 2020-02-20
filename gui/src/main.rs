use grafeia_app::app::App;
use std::alloc::System;

#[global_allocator]
pub static mut THE_ALLOC: System = System;

fn main() {
    env_logger::init();
    let app = if let Some(file) = std::env::args().nth(1) {
        App::import_markdown(&file)
    } else {
        App::load().unwrap_or_else(App::build)
    };

    let data = app.export_docx();
    std::fs::write("document.docx", data).unwrap();

    pathfinder_view::show(app, pathfinder_view::Config {
        zoom: true,
        pan: true
    });
}
