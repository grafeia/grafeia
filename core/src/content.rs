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
use pathfinder_renderer::scene::Scene;
use pathfinder_geometry::rect::RectF;
use serde::{Serialize, Deserialize, Serializer};

use crate::layout::FlexMeasure;
use crate::{
    units::{Length, Bounds, Rect},
    Display, Color,
    gen::GenIter,
    Object
};
use crate::storage::{WordKey, SymbolKey, TargetKey, TypeKey, FontFaceKey, Storage, ObjectKey};

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
    Sequence(Box<Sequence>), // want a box here to keep the type size down
    Object(ObjectKey)
}
impl Item {
    pub fn num_nodes(&self) -> usize {
        match *self {
            Item::Sequence(ref seq) => seq.num_nodes + 2,
            _ => 1,
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
        'a: loop {
            if idx == 0 {
                break;
            }
            idx -= 1;

            for (i, item) in self.items.iter_mut().enumerate() {
                if idx == 0 {
                    f(self, i);
                    break 'a;
                }
                match item {
                    Item::Sequence(ref mut seq) => {
                        if idx < seq.num_nodes + 2 {
                            seq.apply(idx, f);
                            break 'a;
                        }
                        idx -= seq.num_nodes + 2;
                    }
                    _ => {
                        idx -= 1;
                    }
                }
            }

            if idx == 0 {
                f(self, self.items.len());
            }
            break;
        }
        self.num_nodes = self.items.iter().map(|item| item.num_nodes()).sum();
    }
}
#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct Document {
    root: Sequence
}
#[derive(Debug)]
pub enum FindResult<'a> {
    SequenceStart(&'a Sequence),
    SequenceEnd(&'a Sequence),
    Item(&'a Sequence, &'a Item)
}
impl Document {
    pub fn new(root: Sequence) -> Self {
        let mut doc = Document { root };
        doc.validate();
        doc
    }
    // parent and item
    pub fn find(&self, Tag(mut idx): Tag) -> Option<FindResult> {
        let mut seq = &self.root;
        'a: loop {
            if idx == 0 {
                return Some(FindResult::SequenceStart(seq));
            }
            idx -= 1;

            for item in seq.items.iter() {
                match *item {
                    Item::Sequence(ref s) => {
                        if idx < s.num_nodes + 2 {
                            seq = s;
                            continue 'a;
                        }
                        idx -= s.num_nodes + 1;
                    }
                    _ if idx == 0 => {
                        return Some(FindResult::Item(seq, item));
                    }
                    _ =>  {}
                }
                idx -= 1;
            }
            if idx == 0 {
                return Some(FindResult::SequenceEnd(seq));
            }
            return None;
        }
    }
    pub fn root(&self) -> &Sequence {
        &self.root
    }
    pub fn print(&self) {
        let mut level = 0;
        for (idx, r) in self.items(..) {
            println!("{:?} {:?}", idx, r);
            match r {
                FindResult::SequenceStart(_) => level += 1,
                FindResult::SequenceEnd(_) => level -= 1,
                FindResult::Item(_, _) => {}
            }
        }
    }
    #[cfg(not(debug_assertions))]
    pub fn validate(&mut self) {}
    
    #[cfg(debug_assertions)]
    pub fn validate(&mut self) {
        self.print();
        let mut items = vec![];
        for (tag, r1) in self.items(..) {
            let r2 = self.find(tag).unwrap();
            debug!("validating {:?}", tag);
            debug!("r1 = {:?}", r1);
            debug!("r2 = {:?}", r2);
            match (r1, r2) {
                (FindResult::SequenceStart(s1), FindResult::SequenceStart(s2)) => assert_eq!(s1 as *const _, s2 as *const _),
                (FindResult::SequenceEnd(s1), FindResult::SequenceEnd(s2))  => assert_eq!(s1 as *const _, s2 as *const _),
                (FindResult::Item(s1, i1), FindResult::Item(s2, i2)) => {
                    assert_eq!(s1 as *const _, s2 as *const _);
                    assert_eq!(i1 as *const _, i2 as *const _);
                    items.push((tag, i1 as *const _));
                }
                _ => panic!()
            }
        }
        for (tag, item_ptr) in items {
            self.root.apply(tag.0, |seq, i| {
                if let Some(item) = seq.items.get(i) {
                    debug!("[{}] -> {:?}", i, item);
                    assert_eq!(item as *const _, item_ptr);
                }
            });
        }
    }

    pub fn replace(&mut self, Tag(idx): Tag, new_item: Item) {
        self.root.apply(idx, |seq, i| seq.items[i] = new_item);
        self.validate();
    }

    pub fn remove(&mut self, Tag(idx): Tag) {
        self.root.apply(idx, |seq, i| { seq.items.remove(i); });
        self.validate();
    }

    pub fn insert(&mut self, Tag(idx): Tag, new_item: Item) {
        self.root.apply(idx, |seq, i| seq.items.insert(i, new_item));
        self.validate();
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
    pub fn items(&self, range: impl RangeBounds<Tag>) -> impl Iterator<Item=(Tag, FindResult)> {
        use std::ops::Bound;
        let start = match range.start_bound() {
            Bound::Included(&Tag(idx)) => idx,
            Bound::Excluded(&Tag(idx)) => idx + 1,
            Bound::Unbounded => 0
        };
        let end = match range.end_bound() {
            Bound::Included(&Tag(idx)) => idx + 1,
            Bound::Excluded(&Tag(idx)) => idx,
            Bound::Unbounded => self.root.num_nodes + 2
        };

        GenIter::new(move || {
            let mut stack = vec![];
            let mut seq = &self.root;
            let mut idx = 0;
            let mut i = 0;

            yield (Tag(idx), FindResult::SequenceStart(seq));
            idx += 1;

            loop {
                let item = match seq.items.get(i) {
                    Some(item) => item,
                    None => {
                        if idx >= end {
                            return;
                        }
                        yield (Tag(idx), FindResult::SequenceEnd(seq));

                        if let Some((s, j)) = stack.pop() {
                            seq = s;
                            i = j + 1;
                            idx += 1;

                            continue;
                        } else {
                            return;
                        }
                    }
                };
                
                if idx >= end {
                    return;
                }
                
                match *item {
                    Item::Sequence(ref s) => {
                        if idx >= start {
                            yield (Tag(idx), FindResult::SequenceStart(s));
                        }
                        idx += 1;
                        if idx < end {
                            stack.push((seq, i));
                            seq = s;
                            i = 0;
                            continue;
                        }
                    }
                    _ => {
                        if idx >= start {
                            yield (Tag(idx), FindResult::Item(seq, item));
                        }
                        idx += 1;
                    }
                }
                i += 1;
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
