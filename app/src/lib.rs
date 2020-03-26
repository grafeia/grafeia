#[macro_use]
extern crate log;
pub mod app;
pub mod net;

#[cfg(target_arch="wasm32")]
mod browser;

#[cfg(not(target_arch="wasm32"))]
mod desktop;
