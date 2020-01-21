#![feature(generators, generator_trait, entry_insert)]

pub mod content;
pub mod layout;
pub mod units;
pub mod hyphenation;
pub mod builder;
pub mod draw;
mod gen;
mod text;

#[derive(Debug, Copy, Clone)]
pub struct Color;

#[derive(Debug, Copy, Clone)]
pub enum Display {
    Block,
    Inline,

    // Indent
    Paragraph(units::Length)
}
