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

use slab::Slab;
use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::borrow::Borrow;
use font;
use pathfinder_content::outline::Outline;

use crate::layout::FlexMeasure;
use crate::{
    units::{Length, Bounds, Rect},
    Display, Color
};

// possible design and information what it means
// for example: plain text, a bullet list, a heading
pub struct Type {
    pub description: String,
}

pub type FontFace = Box<dyn font::Font<Outline>>;

#[derive(Hash, Eq, PartialEq)]
pub struct Symbol {
    pub text: String
}
impl Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        &*self.text
    }
}

#[derive(Hash, Eq, PartialEq)]
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
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub struct $ty(usize);
    }
}

key!(WordKey);
key!(StringKey);
key!(SymbolKey);
key!(TypeKey);
key!(FontFaceKey);

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
    pub fn find(&self, idx: usize) -> Option<&Item> {
        if idx == 0 {
            return Some(self);
        }
        match *self {
            Item::Sequence(ref seq) => {
                if idx == seq.num_nodes + 1 {
                    return Some(self);
                }
                seq.find(idx - 1)
            },
            _ => None
        }
    }
}

#[derive(Debug)]
pub struct Attribute;

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
    pub fn find(&self, mut idx: usize) -> Option<&Item> {
        for item in self.items() {
            if idx == 0 {
                return Some(item);
            }
            idx -= 1;
            match item {
                Item::Word(_) | Item::Symbol(_) => {}
                Item::Sequence(ref seq) => {
                    if idx < seq.num_nodes {
                        return seq.find(idx);
                    }
                    idx -= seq.num_nodes;
                    if idx == 0 {
                        return Some(item);
                    }
                    idx -= 1;
                }
            }
        }
        None
    }
}

pub struct Storage {
    words:   IndexSet<Word>,
    symbols: IndexSet<Symbol>,
    types:   IndexMap<String, Type>,
    fonts:   Slab<FontFace>,
    targets: Slab<Target>
}
impl Storage {
    pub fn new() -> Storage {
        Storage {
            words:   IndexSet::new(),
            symbols: IndexSet::new(),
            types:   IndexMap::new(),
            fonts:   Slab::new(),
            targets: Slab::new()
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
        let idx= self.fonts.insert(font_face);
        FontFaceKey(idx)
    }

    pub fn get_word(&self, key: WordKey) -> &Word {
        self.words.get_index(key.0).unwrap()
    }
    pub fn get_symbol(&self, key: SymbolKey) -> &Symbol {
        self.symbols.get_index(key.0).unwrap()
    }
    pub fn get_font_face(&self, key: FontFaceKey) -> &FontFace {
        self.fonts.get(key.0).unwrap()
    }
    pub fn find_type(&self, name: &str) -> Option<TypeKey> {
        self.types.get_full(name).map(|(idx, _, _)| TypeKey(idx))
    }
}

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
    pub fn default(&self) -> &TypeDesign {
        &self.default
    }
}

#[derive(Debug, Clone)]
pub struct TypeDesign {
    pub display:        Display,
    pub font:           Font,
    pub word_space:     FlexMeasure,
    pub line_height:    Length
}

// this is a font. it contains all baked in settings
// (font face, size, adjustments…)
#[derive(Debug, Clone)]
pub struct Font {
    pub font_face: FontFaceKey,
    pub size: Length // height of 1em
}


/// Describes a physical print target.
/// The author usually has only few choices here
/// as the parameters are given by the printer
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
