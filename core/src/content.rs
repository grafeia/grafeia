/*
We want a minimal set of items to describe every possible document

What needs to be possible
- config values
- text (duh)
- references
- group (section, paragraph)


Target descriptor
- physical size
- print margins
- colorspace

Design
- font size
- margins
*/

use std::collections::{HashMap};
use std::fmt::{Debug};
use std::borrow::Borrow;
use std::io;
use std::path::Path;
use std::ops::{Deref};
use pathfinder_content::outline::Outline;
use serde::{Serialize, Deserialize, Serializer};

use crate::layout::FlexMeasure;
use crate::{
    units::{Length, Rect},
    Display, Color,
};
use crate::storage::*;

// possible design and information what it means
// for example: plain text, a bullet list, a heading
#[derive(Serialize, Deserialize)]
pub struct Type {
    pub description: String,
}
impl Type {
    pub fn new(description: impl Into<String>) -> Self {
        Type { description: description.into() }
    }
}

#[derive(Deserialize)]
#[serde(from="Vec<u8>")]
pub struct FontFace {
    data: Vec<u8>,
    face: Box<dyn font::Font<Outline>>
}
impl From<Vec<u8>> for FontFace {
    fn from(data: Vec<u8>) -> Self {
        let face = font::parse(&data);
        FontFace { data, face }
    }
}
impl FontFace {
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self::from(std::fs::read(path)?))
    }
}
impl Serialize for FontFace {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.data)
    }
}
impl Deref for FontFace {
    type Target = dyn font::Font<Outline>;
    fn deref(&self) -> &Self::Target {
        &*self.face
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Hash, Eq, PartialEq)]
pub struct Symbol {
    pub text: String
}
impl Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        &*self.text
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Word {
    pub text: String
}
impl Borrow<str> for Word {
    fn borrow(&self) -> &str {
        &*self.text
    }
}


#[derive(Serialize, Deserialize)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Item {
    Word(WordKey),
    Symbol(SymbolKey),
    Sequence(SequenceKey),
    Object(ObjectKey)
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct Attribute;

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tag {
    pub seq: SequenceKey,
    pub pos: SequencePos,
}

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SequencePos {
    At(usize),
    End
}
impl Tag {
    pub fn seq_and_idx(&self) -> Option<(SequenceKey, usize)> {
        match self.pos {
            SequencePos::At(idx) => Some((self.seq, idx)),
            SequencePos::End => None
        }
    }
    pub fn end(seq: SequenceKey) -> Tag {
        Tag {
            seq,
            pos: SequencePos::End
        }
    }
    pub fn at(seq: SequenceKey, idx: usize) -> Tag {
        Tag {
            seq,
            pos: SequencePos::At(idx)
        }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct Sequence {
    pub(crate) typ:   TypeKey,
    pub(crate) items: Vec<Item>,
    pub(crate) attrs: Vec<Attribute>
}
impl Sequence {
    pub fn new(typ: TypeKey, items: Vec<Item>) -> Sequence {
        Sequence { typ, items, attrs: vec![] }
    }
    pub fn items(&self) -> &[Item] {
        &self.items
    }
    pub fn typ(&self) -> TypeKey {
        self.typ
    }
}


#[derive(Serialize, Deserialize)]
pub struct Design {
    name: String,
    map: HashMap<TypeKey, TypeDesign>,
    default: TypeDesign
}
impl Design {
    pub fn new(name: String, default: TypeDesign) -> Self {
        Design {
            name,
            map: HashMap::new(),
            default
        }
    }
    pub fn set_type(&mut self, key: TypeKey, value: TypeDesign) {
        self.map.insert(key, value);
    }
    pub fn get_type(&self, key: TypeKey) -> Option<&TypeDesign> {
        self.map.get(&key)
    }
    pub fn get_type_or_default(&self, key: TypeKey) -> &TypeDesign {
        self.map.get(&key).unwrap_or(&self.default)
    }
    pub fn default(&self) -> &TypeDesign {
        &self.default
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct TypeDesign {
    pub display:        Display,
    pub font:           Font,
    pub word_space:     FlexMeasure,
    pub line_height:    Length
}

// this is a font. it contains all baked in settings
// (font face, size, adjustments…)
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Font {
    pub font_face: FontFaceKey,
    pub size: Length // height of 1em
}


/// Describes a physical print target.
/// The author usually has only few choices here
/// as the parameters are given by the printer
#[derive(Serialize, Deserialize)]
pub struct Target {
    // user visible string
    pub description: String,

    // area where important stuff can be placed
    pub content_box: Rect,

    // area of the entire media (we want to extend graphics that clip the media box to here)
    pub media_box: Rect,

    // the printed media gets trimmed to this
    pub trim_box: Rect,

    // the color of an empty page.
    // we want to know this to show a properly preview
    pub page_color: Color,
}

/*
pub fn hyphenate(w: &mut Writer, word: Word, hyphenator: &Hyphenator) {
    if let Some(points) = hyphenator.get(word.text) {
        w.branch(&mut |b| {
            for p in points.iter() {
                let (left, right) = p.apply(word.text);
                b.add(&mut |w: &mut Writer| {
                    w.word(Atom {
                        left:   word.left,
                        right:  Glue::None,
                        text:   left
                    });
                    w.punctuation(Atom {
                        left:   Glue::None,
                        right:  Glue::newline(),
                        text:   "-"
                    });
                    w.word(Atom {
                        left:   Glue::newline(),
                        right:  word.right,
                        text:   right
                    });
                });
            }
        });
    } else {
        // fallback
        self.word(word);
    }
}
*/
