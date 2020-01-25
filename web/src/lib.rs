#![feature(panic_info_message)]
#[macro_use] extern crate log;

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

#[wasm_bindgen(start)]
pub fn run() {
    use std::panic::PanicInfo;
    std::panic::set_hook(Box::new(|info: &PanicInfo| {
        if let Some(args) = info.message() {
            log(&format!("panic: {}", args));
        }
        else if let Some(s) = info.payload().downcast_ref::<&str>() {
            log(s);
        } else {
            log("panic!");
        }
    }) as _);

    log(&format!("logger: {:p}", log::logger() as *const Log));
    console_log::init_with_level(Level::Trace).unwrap();
    log(&format!("logger: {:p}", log::logger() as *const Log));

    info!("test");

    view::show(App::load().unwrap_or_else(App::build));
}

