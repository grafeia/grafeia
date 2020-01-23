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

use slotmap::SlotMap;
use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::borrow::Borrow;
use std::io;
use std::path::Path;
use std::ops::Deref;
use font;
use pathfinder_content::outline::Outline;
use serde::{Serialize, Deserialize, Serializer};

use crate::layout::FlexMeasure;
use crate::{
    units::{Length, Bounds, Rect},
    Display, Color
};

// possible design and information what it means
// for example: plain text, a bullet list, a heading
#[derive(Serialize, Deserialize)]
pub struct Type {
    pub description: String,
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

macro_rules! key {
    ($ty:ident) => {
        #[derive(Serialize, Deserialize)]
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub struct $ty(usize);
    }
}

key!(WordKey);
key!(StringKey);
key!(SymbolKey);
key!(TypeKey);
new_key_type! {
    pub struct FontFaceKey;
    pub struct TargetKey;
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum Item {
    Word(WordKey),
    Symbol(SymbolKey),
    Sequence(Box<Sequence>) // want a box here to keep the type size down
}
impl Item {
    pub fn num_nodes(&self) -> usize {
        match *self {
            Item::Word(_) | Item::Symbol(_) => 1,
            Item::Sequence(ref seq) => seq.num_nodes + 2
        }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct Attribute;

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tag(pub(crate) usize);

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct Sequence {
    typ:   TypeKey,
    items: Vec<Item>,
    attrs: Vec<Attribute>,
    num_nodes: usize
}
impl Sequence {
    pub fn new(typ: TypeKey, items: Vec<Item>) -> Sequence {
        let num_nodes = items.iter().map(|item| item.num_nodes()).sum();
        Sequence { typ, items, attrs: vec![], num_nodes }
    }
    pub fn items(&self) -> &[Item] {
        &self.items
    }
    pub fn typ(&self) -> TypeKey {
        self.typ
    }
    pub fn num_nodes(&self) -> usize {
        self.num_nodes
    }
    // parent and item
    pub fn find(&self, Tag(mut idx): Tag) -> Option<(&Sequence, &Item)> {
        for item in self.items.iter() {
            if idx == 0 {
                return Some((self, item));
            }
            idx -= 1;
            match item {
                Item::Word(_) | Item::Symbol(_) => {}
                Item::Sequence(ref seq) => {
                    if idx < seq.num_nodes {
                        return seq.find(Tag(idx));
                    }
                    idx -= seq.num_nodes;
                    if idx == 0 {
                        return Some((self, item));
                    }
                    idx -= 1;
                }
            }
        }
        None
    }

    pub fn replace(&mut self, Tag(mut idx): Tag, new_item: Item) {
        use std::mem::replace;
        for item in self.items.iter_mut() {
            if idx == 0 {
                replace(item, new_item);
                break;
            }
            idx -= 1;
            match item {
                Item::Word(_) | Item::Symbol(_) => {}
                Item::Sequence(ref mut seq) => {
                    if idx < seq.num_nodes {
                        seq.replace(Tag(idx), new_item);
                        break;
                    }
                    idx -= seq.num_nodes;
                    if idx == 0 {
                        replace(item, new_item);
                        break;
                    }
                    idx -= 1;
                }
            }
        }

        self.num_nodes = self.items.iter().map(|item| item.num_nodes()).sum();
    }
}

#[derive(Serialize, Deserialize)]
pub struct Storage {
    words:   IndexSet<Word>,
    symbols: IndexSet<Symbol>,
    types:   IndexMap<String, Type>,
    fonts:   SlotMap<FontFaceKey, FontFace>,
    targets: SlotMap<TargetKey, Target>
}
impl Storage {
    pub fn new() -> Storage {
        Storage {
            words:   IndexSet::new(),
            symbols: IndexSet::new(),
            types:   IndexMap::new(),
            fonts:   SlotMap::with_key(),
            targets: SlotMap::with_key()
        }
    }
    pub fn insert_word(&mut self, text: &str) -> WordKey {
        if let Some((idx, _)) = self.words.get_full(text) {
            return WordKey(idx);
        }
        let (idx, _) = self.words.insert_full(Word { text: text.to_owned() });
        WordKey(idx)
    }
    pub fn insert_symbol(&mut self, text: &str) -> SymbolKey {
        if let Some((idx, _)) = self.symbols.get_full(text) {
            return SymbolKey(idx);
        }
        let (idx, _) = self.symbols.insert_full(Symbol { text: text.to_owned() });
        SymbolKey(idx)
    }
    pub fn insert_type(&mut self, key: String, typ: Type) -> TypeKey {
        let (idx, _) = self.types.insert_full(key, typ);
        TypeKey(idx)
    }
    pub fn insert_font_face(&mut self, font_face: FontFace) -> FontFaceKey {
        self.fonts.insert(font_face)
    }

    pub fn get_word(&self, key: WordKey) -> &Word {
        self.words.get_index(key.0).unwrap()
    }
    pub fn get_symbol(&self, key: SymbolKey) -> &Symbol {
        self.symbols.get_index(key.0).unwrap()
    }
    pub fn get_font_face(&self, key: FontFaceKey) -> &FontFace {
        self.fonts.get(key).unwrap()
    }
    pub fn find_type(&self, name: &str) -> Option<TypeKey> {
        self.types.get_full(name).map(|(idx, _, _)| TypeKey(idx))
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
