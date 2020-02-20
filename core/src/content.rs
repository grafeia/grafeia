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
use std::fmt::{self, Debug};
use std::borrow::Borrow;
use std::io;
use std::path::Path;
use std::ops::{Deref};
use std::sync::Arc;
use pathfinder_content::outline::Outline;
use serde::{Serialize, Deserialize, Serializer};

use crate::layout::FlexMeasure;
use crate::{*};

// possible design and information what it means
// for example: plain text, a bullet list, a heading
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Type {
    pub description: String,
}
impl Type {
    pub fn new(description: impl Into<String>) -> Self {
        Type { description: description.into() }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FontFace(Arc<FontFaceInner>);

#[derive(Deserialize)]
#[serde(from="Vec<u8>")]
struct FontFaceInner {
    data: Vec<u8>,

    #[serde(skip)]
    face: Box<dyn font::Font<Outline> + Send + Sync + 'static>
}

impl From<Vec<u8>> for FontFaceInner {
    fn from(data: Vec<u8>) -> Self {
        let face = font::parse::<Outline>(&data);
        FontFaceInner { data, face }
    }
}
impl FontFace {
    pub fn from_data(data: Vec<u8>) -> Self {
        FontFace(Arc::new(data.into()))
    }
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let inner = FontFaceInner::from(std::fs::read(path)?);
        Ok(FontFace(Arc::new(inner)))
    }
}
impl Debug for FontFace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Font({})", self.0.face.full_name().unwrap_or(""))
    }
}
impl Serialize for FontFaceInner {
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
        &*self.0.face
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct Symbol {
    pub text: String
}
impl Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        &*self.text
    }
}

pub enum Direction {
    LeftToRight,
    RightToLeft
}

#[derive(Serialize, Deserialize)]
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Word {
    pub text: String
}
impl Borrow<str> for Word {
    fn borrow(&self) -> &str {
        &*self.text
    }
}


#[derive(Serialize, Deserialize)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum Item {
    Word(WordId),
    Symbol(SymbolId),
    Sequence(SequenceId),
    Object(ObjectId)
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct Attribute;

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tag {
    Start(SequenceId),
    Item(SequenceId, Id),
    End(SequenceId)
}
impl Tag {
    pub fn seq(&self) -> SequenceId {
        match *self {
            Tag::Start(s) | Tag::Item(s, _) | Tag::End(s) => s
        }
    }
    pub fn item(&self) -> Option<Id> {
        match *self {
            Tag::Item(_, i) => Some(i),
            _ => None
        }
    }
    pub fn seq_and_item(&self) -> Option<(SequenceId, Id)> {
        match *self {
            Tag::Item(s, i) => Some((s, i)),
            _ => None
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Design {
    name: String,
    map: HashMap<TypeId, TypeDesign>,
    default: TypeDesign,
}
impl Design {
    pub fn new(name: String, default: TypeDesign) -> Self {
        Design {
            name,
            map: HashMap::new(),
            default,
        }
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn set_type(&mut self, key: TypeId, value: TypeDesign) {
        self.map.insert(key, value);
    }
    pub fn get_type(&self, key: TypeId) -> Option<&TypeDesign> {
        self.map.get(&key)
    }
    pub fn get_type_or_default(&self, key: TypeId) -> &TypeDesign {
        self.map.get(&key).unwrap_or(&self.default)
    }
    pub fn default(&self) -> &TypeDesign {
        &self.default
    }
    pub fn items<'s>(&'s self) -> impl Iterator<Item=(TypeId, &TypeDesign)> + 's {
        self.map.iter().map(|(&k, v)| (k, v))
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
    pub font_face: FontId,
    pub size: Length // height of 1em
}


/// Describes a physical print target.
/// The author usually has only few choices here
/// as the parameters are given by the printer
#[derive(Serialize, Deserialize, Clone)]
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
