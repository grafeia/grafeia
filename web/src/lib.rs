use grafeia_app::app::{App, NetworkApp};
use wasm_bindgen::prelude::*;
use std::panic;
use log::{Log, Level};

#[wasm_bindgen]
extern {
    fn ws_log(msg: &str);
    fn log_err(msg: &str);
}

fn panic_hook(info: &panic::PanicInfo) {
    let mut msg = info.to_string();
    log_err(&msg);

    console_error_panic_hook::hook(info);
}

pub fn log(record: &log::Record) {
    match record.level() {
        Level::Error => log_err(&record.args().to_string()),
        level => ws_log(&format!("{:?} {}", level, record.args()))
    }
}

struct WebsocketLogger;

impl Log for WebsocketLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        log(record);
    }

    fn flush(&self) {}
}

static LOGGER: WebsocketLogger = WebsocketLogger;

#[wasm_bindgen(start)]
pub fn run() {
    panic::set_hook(Box::new(panic_hook));
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);
}

#[wasm_bindgen]
pub fn online() {
    pathfinder_view::show(NetworkApp::new(), pathfinder_view::Config {
        zoom: false,
        pan: true
    });
}

#[wasm_bindgen]
pub fn offline() {
    pathfinder_view::show(App::build(), pathfinder_view::Config {
        zoom: false,
        pan: true
    });
}
