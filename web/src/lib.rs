extern crate console_error_panic_hook;

use grafeia_app::{
    app::App,
    view,
    view::Interactive
};

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    view::show(App::load().unwrap_or_else(App::build));
}

