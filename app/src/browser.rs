use wasm_bindgen::prelude::*;
use pathfinder_view::{Context};

#[wasm_bindgen]
extern {
    fn ws_send(data: &[u8]);
}

pub struct Connection;
impl Connection {
    pub fn init(ctx: &mut Context) -> Self {
        Connection
    }
    pub fn send(&mut self, data: Vec<u8>) {
        ws_send(&data);
    }
}