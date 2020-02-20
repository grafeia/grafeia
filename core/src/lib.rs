#![feature(generators, generator_trait, entry_insert)]

#[macro_use] extern crate slotmap;
use serde::{Serialize, Deserialize};

#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;

pub mod content;
pub mod layout;
pub mod units;
//pub mod hyphenation;
pub mod builder;
pub mod draw;
pub mod net;
mod gen;
mod text;
mod object;
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

#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone)]
pub enum Display {
    Block,
    Inline,

    // Indent
    Paragraph(units::Length)
}
