#![feature(generators, generator_trait, entry_insert)]

use serde::{Serialize, Deserialize};

#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;

macro_rules! data {
    ($path:tt) => (
        &*include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/", $path ))
    )
}

pub mod content;
pub mod layout;
pub mod units;
//pub mod hyphenation;
pub mod builder;
pub mod draw;
pub mod net;
mod gen;
mod text;
pub mod object;
mod document;

pub use content::*;
pub use layout::FlexMeasure;
pub use units::*;
pub use object::*;
pub use document::*;
pub use net::*;

#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone)]
pub struct Color;
