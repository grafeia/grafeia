#[macro_use] extern crate log;
//extern crate console_error_panic_hook;

use grafeia_app::{
    app::App,
    view,
    view::Interactive
};

use wasm_bindgen::prelude::*;
use log::{Log, Level};

fn log(s: &str) {
    web_sys::console::log_1(&JsValue::from_str(s));
}
fn error(s: &str) {
    web_sys::console::error_1(&JsValue::from_str(s));
}

#[wasm_bindgen(start)]
pub fn run() {
    //std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    console_log::init_with_level(Level::Trace).unwrap();

    info!("test");

    view::show(App::load().unwrap_or_else(App::build));
}

