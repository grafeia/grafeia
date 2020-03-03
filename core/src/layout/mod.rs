use std::fmt::Debug;
use crate::{Tag, Length};
use crate::draw::RenderItem;

// private mods
mod glue;
//mod paragraph;
mod writer;
mod flex;
pub mod columns;

pub use self::glue::Glue;
//pub use self::paragraph::ParagraphLayout;
pub use self::writer::{Writer};
pub use self::flex::FlexMeasure;
pub use self::columns::*;

#[derive(Clone)]
pub struct Style {
    pub word_space: FlexMeasure,
}

#[derive(Debug, Clone, Copy)]
pub struct ItemMeasure {
    // normal width
    pub content: FlexMeasure,

    // how much to overflow in to the left margin
    pub left:    FlexMeasure,

    // how much to overflow into the right margin
    pub right:   FlexMeasure,

    pub height:  Length,
}

/// used as input to the line breaking algorithm
#[derive(Debug)]
#[allow(dead_code)]
enum Entry {
    Item(ItemMeasure, RenderItem, Tag),
    
    /// Continue on the next line (fill)
    Linebreak(bool, Length),
    
    /// Indent, Width
    SetWidth(Length, Length),

    /// (measure, breaking)
    Space(FlexMeasure, bool),

    /// Somtimes there are different possiblites of representing something.
    /// A Branch solves this by splitting the stream in two parts.
    /// The default path is taken by skipping the specified amount of entries.
    /// The other one by following the next items.
    ///
    /// normal items
    /// BranchEntry(3)
    ///   branched item 1
    ///   branched item 2
    /// BranchExit(1)
    ///   normal item 1
    /// both sides joined here
    BranchEntry(usize),
    
    /// Each BranchEntry is followed by BranchExit. It specifies the number of
    /// items to skip.
    BranchExit(usize),
}



#[derive(Debug)]
pub struct StreamVec(Vec<Entry>);
impl StreamVec {
    pub fn new() -> Self {
        StreamVec(vec![])
    }
    #[inline]
    fn push(&mut self, entry: Entry) {
        self.0.push(entry);
    }
}
