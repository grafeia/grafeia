use grafeia_app::app::{App, NetworkApp};
use grafeia_core::{State, SiteId};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use std::panic;
use log::{Log, Level};
use pathfinder_view::{WasmView, Config};

#[wasm_bindgen]
extern {
    fn log(level: u8, msg: &str);
}

fn panic_hook(info: &panic::PanicInfo) {
    let msg = info.to_string();
    log(Level::Error as u8, &msg);

    console_error_panic_hook::hook(info);
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

        log(record.level() as u8, &record.args().to_string());
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
pub fn online(canvas: HtmlCanvasElement) -> WasmView {
    WasmView::new(
        canvas,
        Config {
            zoom: false,
            pan: false
        },
        Box::new(NetworkApp::new()) as _
    )
}

#[wasm_bindgen]
pub fn offline(canvas: HtmlCanvasElement, data: &[u8]) -> WasmView {
    let state = State::load(data).unwrap();
    WasmView::new(
        canvas,
        Config {
            zoom: false,
            pan: false
        },
        Box::new(App::from_state(state, SiteId(1))) as _
    )
}
