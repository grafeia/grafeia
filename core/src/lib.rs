#![feature(generators, generator_trait, entry_insert)]

#[macro_use] extern crate slotmap;
use serde::{Serialize, Deserialize};

#[macro_use] extern crate log;

pub mod content;
pub mod layout;
pub mod units;
//pub mod hyphenation;
pub mod builder;
pub mod draw;
mod gen;
mod text;
mod storage;
mod object;

pub use storage::*;
pub use content::*;
pub use layout::FlexMeasure;
pub use units::*;
pub use object::*;

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
