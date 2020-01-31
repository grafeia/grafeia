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
use std::ops::{Deref, RangeBounds};
use font;
use pathfinder_content::outline::Outline;
use serde::{Serialize, Deserialize, Serializer};

use crate::layout::FlexMeasure;
use crate::{
    units::{Length, Bounds, Rect},
    Display, Color,
    gen::GenIter
};
use crate::storage::{WordKey, SymbolKey, TargetKey, TypeKey, FontFaceKey};

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

impl std::ops::Add<usize> for Tag {
    type Output = Tag;
    fn add(self, rhs: usize) -> Tag {
        Tag(self.0 + rhs)
    }
}

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

    fn apply(&mut self, mut idx: usize, f: impl FnOnce(&mut Sequence, usize)) {
        for (i, item) in self.items.iter_mut().enumerate() {
            if idx == 0 {
                f(self, i);
                break;
            }
            idx -= 1;
            match item {
                Item::Word(_) | Item::Symbol(_) => {}
                Item::Sequence(ref mut seq) => {
                    if idx < seq.num_nodes {
                        seq.apply(idx, f);
                        break;
                    }
                    idx -= seq.num_nodes;
                    if idx == 0 {
                        f(self, i);
                        break;
                    }
                    idx -= 1;
                }
            }
        }

        self.num_nodes = self.items.iter().map(|item| item.num_nodes()).sum();
    }

    fn walk(&self, mut f: impl FnMut(&Item)) {
        for item in self.items.iter() {
            f(item);
            match *item {
                Item::Sequence(ref seq) => seq.walk(|item| f(item)),
                _ => {}
            }
        }
    }
}
#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct Document {
    root: Sequence
}
impl Document {
    pub fn new(root: Sequence) -> Self {
        Document { root }
    }
    // parent and item
    pub fn find(&self, Tag(mut idx): Tag) -> Option<(&Sequence, &Item)> {
        let mut seq = &self.root;
        'a: loop {
            for item in seq.items.iter() {
                if idx == 0 {
                    return Some((seq, item));
                }
                idx -= 1;
                match item {
                    Item::Word(_) | Item::Symbol(_) => {}
                    Item::Sequence(ref s) => {
                        if idx < s.num_nodes {
                            seq = s;
                            continue 'a;
                        }
                        idx -= s.num_nodes;
                        if idx == 0 {
                            return Some((s, item));
                        }
                        idx -= 1;
                    }
                }
            }
            return None;
        }
    }
    pub fn root(&self) -> &Sequence {
        &self.root
    }

    pub fn replace(&mut self, Tag(idx): Tag, new_item: Item) {
        self.root.apply(idx, |seq, i| seq.items[i] = new_item)
    }

    pub fn remove(&mut self, Tag(idx): Tag) {
        self.root.apply(idx, |seq, i| { seq.items.remove(i); })
    }

    pub fn insert(&mut self, Tag(idx): Tag, new_item: Item) {
        self.root.apply(idx, |seq, i| seq.items.insert(i, new_item))
    }

    pub fn get_previous_tag(&self, Tag(idx): Tag) -> Option<Tag> {
        if idx > 0 {
            Some(Tag(idx - 1))
        } else {
            None
        }
    }
    pub fn get_next_tag(&self, Tag(idx): Tag) -> Option<Tag> {
        if idx + 1 < self.root.num_nodes {
            Some(Tag(idx + 1))
        } else {
            None
        }
    }
    pub fn items(&self, range: impl RangeBounds<Tag>) -> impl Iterator<Item=(Tag, &Item)> {
        use std::ops::Bound;
        let start = match range.start_bound() {
            Bound::Included(&Tag(idx)) => idx,
            Bound::Excluded(&Tag(idx)) => idx + 1,
            Bound::Unbounded => 0
        };
        let end = match range.end_bound() {
            Bound::Included(&Tag(idx)) => idx + 1,
            Bound::Excluded(&Tag(idx)) => idx,
            Bound::Unbounded => self.root.num_nodes
        };

        GenIter::new(move || {
            let mut stack = vec![];
            let mut seq = &self.root;
            let mut idx = 0;
            let mut i = 0;

            loop {
                let item = match seq.items.get(i) {
                    Some(item) => item,
                    None => {
                        if let Some((s, j)) = stack.pop() {
                            seq = s;
                            i = j;
                            continue;
                        } else {
                            return;
                        }
                    }
                };
                
                if idx >= end {
                    return;
                }

                if idx >= start {
                    yield (Tag(idx), item);
                }

                idx += 1;
                i += 1;

                match *item {
                    Item::Word(_) | Item::Symbol(_) => {}
                    Item::Sequence(ref s) => {
                        if idx <= end {
                            stack.push((seq, i));
                            seq = s;
                            i = 0;
                            continue;
                        }
                    }
                }
            }
        })
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
