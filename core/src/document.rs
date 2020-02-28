use crate::*;
use std::collections::HashMap;
use std::ops::{Deref};
use std::cmp::{PartialEq, PartialOrd};
use std::hash::Hash;
use std::fmt;
use std::borrow::Cow;
use itertools::Itertools;
use unicode_categories::UnicodeCategories;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Serialize, Deserialize)]
#[derive(Hash, Eq, PartialEq, Copy, Clone, Default, PartialOrd, Ord, Debug)]
pub struct SiteId(pub u32);

#[derive(Serialize, Deserialize)]
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Copy, Clone, Hash)]
pub struct Id {
    clock: u32,
    site:  SiteId,
}
impl Id {
    pub const fn null() -> Id {
        Id { clock: 0, site: SiteId(0) }
    }
    pub fn is_null(&self) -> bool {
        *self == Id::null()
    }
}
impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.site.0, self.clock)
    }
}

pub trait ClockValue: Default + Clone + Ord {
    fn inc(self) -> Self;
}
impl ClockValue for u32 {
    fn inc(self) -> Self { self + 1 }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Clock<T>(T);
impl<T> Clock<T> where T: ClockValue {
    pub fn new() -> Self {
        Clock(T::default())
    }
    pub fn next(&mut self) -> T {
        let t = self.0.clone().inc();
        std::mem::replace(&mut self.0, t)
    }
    pub fn seen(&mut self, val: T) {
        self.0 = self.0.clone().max(val);
    }
}

#[derive(Serialize, Deserialize)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone)]
pub struct Atom {
    prev: Id,
    op: AtomOp,
    id: Id,
}

pub trait Stamped<T> {
    fn value(&self) -> T;
    fn site(&self) -> SiteId;
    fn new(site: SiteId, val: T) -> Self;
}

macro_rules! id {
    ($($name:ident),*) => ( $(
        #[derive(Serialize, Deserialize)]
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
        pub struct $name(SiteId, u32);

        impl Stamped<u32> for $name {
            fn new(site: SiteId, n: u32) -> Self {
                $name(site, n)
            }
            fn site(&self) -> SiteId { self.0 }
            fn value(&self) -> u32 { self.1 }
        }
    )* )
}

id!(WordId, SymbolId, ObjectId, SequenceId, TypeId, FontId, DictId);

#[derive(Serialize, Deserialize)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum AtomOp {
    Remove,
    Replace(Item),
    Add(Item),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Weave {
    typ: TypeId,
    atoms: Vec<Atom>,
    clock: Clock<u32>
}
impl Weave {
    pub fn new(typ: TypeId) -> Weave {
        Weave {
            typ,
            atoms: Vec::new(),
            clock: Clock::new()
        }
    }
    pub fn typ(&self) -> TypeId {
        self.typ
    }
    fn from_items(site: SiteId, typ: TypeId, items: impl Iterator<Item=Item>) -> Weave {
        let mut prev = Id::null();
        let mut weave = Weave::new(typ);
        for item in items {
            let atom = weave.create(site, prev, AtomOp::Add(item));
            prev = atom.id;
            weave.add(atom);
        }
        weave
    }
    fn create(&mut self, site: SiteId, prev: Id, op: AtomOp) -> Atom {
        let id = Id { site, clock: self.clock.next() };
        Atom {
            prev,
            id,
            op
        }
    }

    fn add(&mut self, atom: Atom) {
        let mut idx = if atom.prev.is_null() {
            0
        } else {
            self.atoms.iter().position(|other| other.id == atom.prev).expect("previous item not found") + 1
        };

        while let Some(&other) = self.atoms.get(idx) {
            // unrelated item. insert before
            if atom.prev != other.prev || atom < other {
                break;
            }

            // skip child atoms
            let mut prev_id = other.id;
            idx += 1;
            while let Some(other) = self.atoms.get(idx) {
                if other.prev == prev_id {
                    prev_id = other.id;
                    idx += 1;
                } else {
                    break;
                }
            }
        }

        self.atoms.insert(idx, atom);
        self.clock.seen(atom.id.clock);
    }
    pub fn items<'s>(&'s self) -> impl Iterator<Item=(Id, Item)> + 's {
        use crate::gen::GenIter;

        let mut pending: Option<(Id, Item)> = None;
        GenIter::new(move || {
            for atom in self.atoms.iter() {
                match atom.op {
                    AtomOp::Add(item) => {
                        if let Some((id, item)) = pending.replace((atom.id, item)) {
                            yield (id, item);
                        }
                    }
                    AtomOp::Replace(item) => {
                        match pending {
                            Some((prev, _)) if prev == atom.prev => pending = Some((atom.id, item)),
                            _ => {}
                        }
                    }
                    AtomOp::Remove => {
                        // we clear last_id when removing an item, so it can only happen at most once for each item
                        match pending {
                            Some((prev, _)) if prev == atom.prev => pending = None,
                            _ => {}
                        }
                    }
                }
            }
            if let Some((id, item)) = pending.take() {
                yield (id, item);
            }
        })
    }
    pub fn render<'s>(&'s self) -> impl Iterator<Item=Item> + 's {
        self.items().map(|(_, item)| item)
    }
    pub fn get_item(&self, id: Id) -> Option<Item> {
        self.atoms.iter().find(|atom| atom.id == id)
        .and_then(|atom| match atom.op {
            AtomOp::Add(item) | AtomOp::Replace(item) => Some(item),
            _ => None
        })
    }
    pub fn get_previous(&self, id: Id) -> Option<(Id, Item)> {
        self.items().tuple_windows().find(|&(_, b)| b.0 == id).map(|(a, _)| a)
    }
    pub fn get_next(&self, id: Id) -> Option<(Id, Item)> {
        self.items().tuple_windows().find(|&(a, _)| a.0 == id).map(|(_, b)| b)
    }
    pub fn get_first(&self) -> Option<(Id, Item)> {
        self.items().next()
    }
    pub fn get_last(&self) -> Option<(Id, Item)> {
        self.items().last()
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Map<K: Eq + Hash, V> {
    map: HashMap<K, V>,
    clock: Clock<u32>,
}
impl<K: Eq + Hash + Copy + Stamped<u32>, V> Map<K, V> {
    pub fn new() -> Self {
        Map {
            map: HashMap::new(),
            clock: Clock::new()
        }
    }
    pub fn create(&mut self, site: SiteId, value: V) -> K {
        let key = K::new(site, self.clock.next());
        self.map.insert(key, value);
        key
    }
    pub fn insert(&mut self, key: K, value: V) {
        self.map.insert(key, value);
        self.clock.seen(key.value());
    }
    pub fn get(&self, key: K) -> Option<&V> {
        self.map.get(&key)
    }
    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.map.get_mut(&key)
    }
    pub fn iter(&self) -> impl Iterator<Item=(K, &V)> {
        self.map.iter().map(|(&k, v)| (k, v))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DocumentOp {
    SeqOp(SequenceId, Atom),
    CreateSequence(SequenceId, TypeId),
    CreateWord(WordId, Word),
    CreateSymbol(SymbolId, Symbol),
    CreateType(TypeId, String, Type),
    CreateFont(FontId, FontFace),
    CreateObject(ObjectId, Object),
    CreateDictionary(DictId, Dictionary),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Storage {
    weaves:  Map<SequenceId, Weave>,
    words:   Map<WordId,     Word>,
    symbols: Map<SymbolId,   Symbol>,
    objects: Map<ObjectId,   Object>,
    types:   Map<TypeId,     Type>,
    type_names: HashMap<String, TypeId>,
    fonts:   Map<FontId,     FontFace>,
    dicts:   Map<DictId,     Dictionary>,
    sites:   Clock<u32>
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            weaves: Map::new(),
            words: Map::new(),
            symbols: Map::new(),
            objects: Map::new(),
            types: Map::new(),
            type_names: HashMap::new(),
            fonts: Map::new(),
            dicts: Map::new(),
            sites: Clock::new()
        }
    }
    pub fn apply(&mut self, doc_op: DocumentOp) {
        match doc_op {
            DocumentOp::SeqOp(id, op) => {
                self.weaves.get_mut(id).unwrap().add(op);
            }
            DocumentOp::CreateSequence(id, typ) => {
                self.weaves.insert(id, Weave::new(typ));
            }
            DocumentOp::CreateWord(id, word) => {
                self.words.insert(id, word);
            }
            DocumentOp::CreateSymbol(id, symbol) => {
                self.symbols.insert(id, symbol);
            }
            DocumentOp::CreateObject(id, object) => {
                self.objects.insert(id, object);
            }
            DocumentOp::CreateFont(id, font) => {
                self.fonts.insert(id, font);
            }
            DocumentOp::CreateType(id, name, typ) => {
                self.types.insert(id, typ);
                self.type_names.insert(name, id);
            }
            DocumentOp::CreateDictionary(id, dict) => {
                self.dicts.insert(id, dict);
            }
        }
    }

    pub fn log_weave(&self, id: SequenceId) {
        let weave = self.weaves.get(id).unwrap();
        let item = |item: Item| match item {
            Item::Word(key) => self.words.get(key).unwrap().text.as_str(),
            Item::Symbol(key) => self.symbols.get(key).unwrap().text.as_str(),
            Item::Sequence(_) => "<seq>",
            Item::Object(_) => "<obj>",
        };

        for atom in weave.atoms.iter() {
            match atom.op {
                AtomOp::Remove => info!("{} Remove {}", atom.id, atom.prev),
                AtomOp::Replace(id) => info!("{} Replace({}) {}", atom.id, item(id), atom.prev),
                AtomOp::Add(id) => info!("{} Add({}) {}", atom.id, item(id), atom.prev),
            }
        }
    }
    pub fn get_item(&self, tag: Tag) -> Option<Item> {
        let weave = self.weaves.get(tag.seq())?;
        weave.get_item(tag.item()?)
    }
    pub fn get_word(&self, id: WordId) -> &Word {
        self.words.get(id).unwrap()
    }
    pub fn get_symbol(&self, id: SymbolId) -> &Symbol {
        self.symbols.get(id).unwrap()
    }
    pub fn get_object(&self, id: ObjectId) -> &Object {
        self.objects.get(id).unwrap()
    }
    pub fn get_font_face(&self, id: FontId) -> &FontFace {
        self.fonts.get(id).unwrap()
    }
    pub fn get_weave(&self, id: SequenceId) -> &Weave {
        self.weaves.get(id).unwrap()
    }
    pub fn get_dict(&self, id: DictId) -> &Dictionary {
        self.dicts.get(id).unwrap()
    }

    pub fn get_last(&self, seq_id: SequenceId) -> Option<(Tag, Item)> {
        self.weaves.get(seq_id).unwrap().items()
            .last()
            .map(|(item_id, item)| (Tag::Item(seq_id, item_id), item))
    }
    pub fn get_first(&self, seq_id: SequenceId) -> Option<(Tag, Item)> {
        self.weaves.get(seq_id).unwrap().items().next()
            .map(|(item_id, item)| (Tag::Item(seq_id, item_id), item))
    }
    pub fn find_type(&self, name: &str) -> Option<TypeId> {
        self.type_names.get(name).cloned()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct State<'a> {
    pub target: Cow<'a, Target>,
    pub design: Cow<'a, Design>,
    pub storage: Cow<'a, Storage>,
    pub root: SequenceId
}
impl<'a> State<'a> {
    pub fn borrowed<'b: 'a>(&'b self) -> State<'b> {
        use std::borrow::Borrow;
        State {
            target: Cow::Borrowed(self.target.borrow()),
            design: Cow::Borrowed(self.design.borrow()),
            storage: Cow::Borrowed(self.storage.borrow()),
            root: self.root
        }
    }
}

pub struct Document {
    storage: Storage,
    site: SiteId,
    pending: Vec<DocumentOp>,
    parents: HashMap<SequenceId, Tag>,
    words: HashMap<String, WordId>,
    symbols: HashMap<String, SymbolId>,
    root: Option<SequenceId>,
}
impl Deref for Document {
    type Target = Storage;
    fn deref(&self) -> &Storage {
        &self.storage
    }
}

impl Document {
    pub fn new(storage: Storage) -> Document {
        let site = SiteId(1);
        let pending = Vec::new();
        let parents = HashMap::new();
        let words = HashMap::new();
        let symbols = HashMap::new();

        Document {
            storage,
            site,
            pending,
            parents,
            words,
            symbols,
            root: None,
        }
    }
    pub fn set_root(&mut self, root: SequenceId) {
        self.root = Some(root);
    }
    pub fn from_storage(storage: Storage, root: SequenceId, site: SiteId) -> Document {
        let mut parents = HashMap::new();
        for (seq_id, weave) in storage.weaves.iter() {
            for (item_id, item) in weave.items() {
                if let Item::Sequence(child_id) = item {
                    parents.insert(child_id, Tag::Item(seq_id, item_id));
                }
            }
        }
        
        let words = storage.words.iter().map(|(id, word)| (word.text.clone(), id)).collect();
        let symbols = storage.symbols.iter().map(|(id, symbol)| (symbol.text.clone(), id)).collect();

        Document {
            site,
            storage,
            pending: Vec::new(),
            parents,
            words,
            symbols,
            root: Some(root)
        }
    }
    pub fn root(&self) -> SequenceId {
        self.root.expect("root not set")
    }
    pub fn into_storage(self) -> Storage {
        self.storage
    }
    pub fn storage(&self) -> &Storage {
        &self.storage
    }
    pub fn drain_pending<'s>(&'s mut self) -> impl Iterator<Item=DocumentOp> + 's {
        self.pending.drain(..)
    }

    pub fn exec_op(&mut self, op: DocumentOp) {
        match op {
            DocumentOp::SeqOp(id, atom) => {
                let weave = self.storage.weaves.get_mut(id).unwrap();
                weave.add(atom);
                match atom.op {
                    AtomOp::Add(Item::Sequence(_)) | AtomOp::Replace(Item::Sequence(_)) => {
                        self.link(Tag::Item(id, atom.id));
                    }
                    _ => {}
                }
            }
            DocumentOp::CreateWord(id, ref word) => {
                self.words.insert(word.text.clone(), id);
            }
            DocumentOp::CreateSymbol(id, ref symbol) => {
                self.symbols.insert(symbol.text.clone(), id);
            }
            _ => {}
        }
        self.storage.apply(op);
    }

    pub fn replace(&mut self, tag: Tag, new_item: Item) -> Tag {
        self.unlink(tag);
        let (seq, id) = tag.seq_and_item().unwrap();

        let weave = self.storage.weaves.get_mut(seq).unwrap();
        let atom = weave.create(self.site, id, AtomOp::Replace(new_item));
        weave.add(atom);
        self.storage.log_weave(seq);
        self.pending.push(DocumentOp::SeqOp(seq, atom));

        self.link(tag);
        Tag::Item(seq, atom.id)
    }

    pub fn remove(&mut self, tag: Tag) {
        self.unlink(tag);
        let (seq, id) = tag.seq_and_item().unwrap();

        let weave = self.storage.weaves.get_mut(seq).unwrap();
        let atom = weave.create(self.site, id, AtomOp::Remove);
        weave.add(atom);

        info!("remove {} from {:?}", id, seq);
        self.storage.log_weave(seq);
        self.pending.push(DocumentOp::SeqOp(seq, atom));
    }

    pub fn insert(&mut self, tag: Tag, new_item: Item) -> Tag {
        let (seq, prev_id) = match tag {
            Tag::Start(seq) => (seq, Id::null()),
            Tag::Item(seq, item) => (seq, item),
            Tag::End(_) => panic!("not a valid insert location")
        };
        info!("insert {:?} at {} into {:?}", new_item, prev_id, seq);
        let weave = self.storage.weaves.get_mut(seq).unwrap();

        let atom = weave.create(self.site, prev_id, AtomOp::Add(new_item));
        weave.add(atom);
        let tag = Tag::Item(seq, atom.id);
        self.storage.log_weave(seq);
        self.pending.push(DocumentOp::SeqOp(seq, atom));

        self.link(tag);
        tag
    }

    pub fn create_word(&mut self, text: &str) -> WordId {
        match self.words.get(text) {
            Some(&id) => id,
            None => {
                let word = Word { text: text.into() };
                let id = self.storage.words.create(self.site, word.clone());
                self.words.insert(text.into(), id);
                self.pending.push(DocumentOp::CreateWord(id, word));
                id
            }
        }
    }

    pub fn crate_seq(&mut self, typ: TypeId) -> SequenceId {
        let id = self.storage.weaves.create(self.site, Weave::new(typ));
        self.pending.push(DocumentOp::CreateSequence(id, typ));
        id
    }
    pub fn creat_seq_with_items(&mut self, typ: TypeId, items: impl IntoIterator<Item=Item>) -> SequenceId {
        let id = self.storage.weaves.create(self.site, Weave::from_items(self.site, typ, items.into_iter()));
        self.pending.push(DocumentOp::CreateSequence(id, typ));
        id
    }
    pub fn childen<'s>(&'s self, parent: SequenceId) -> impl Iterator<Item=Tag> + 's {
        use crate::gen::GenIter;
        let storage = &self.storage;
        GenIter::new(move || {
            let mut stack = vec![];
            let traverse = move |key| (key, storage.weaves.get(parent).unwrap().items());

            let mut current = traverse(parent);

            loop {
                while let Some((item_id, item)) = current.1.next() {
                    if let Item::Sequence(child) = item {
                        stack.push(std::mem::replace(&mut current, traverse(child)));
                        continue;
                    } else {
                        yield Tag::Item(current.0, item_id);
                    }
                }
                if let Some(parent_iter) = stack.pop() {
                    current = parent_iter;
                    continue;
                }
                break;
            }
        })
    }

    pub fn create_type(&mut self, name: &str, typ: Type) -> TypeId {
        let id = self.storage.types.create(self.site, typ.clone());
        self.storage.type_names.insert(name.to_owned(), id);
        self.pending.push(DocumentOp::CreateType(id, name.into(), typ));
        id
    }
    pub fn create_object(&mut self, object: Object) -> ObjectId {
        let id = self.storage.objects.create(self.site, object.clone());
        self.pending.push(DocumentOp::CreateObject(id, object));
        id
    }
    pub fn add_font(&mut self, data: impl Into<Vec<u8>>) -> FontId {
        let font = FontFace::from_data(data.into());
        let id = self.storage.fonts.create(self.site, font.clone());
        self.pending.push(DocumentOp::CreateFont(id, font));
        id
    }
    pub fn add_symbol(&mut self, symbol: Symbol) -> SymbolId {
        let id = self.storage.symbols.create(self.site, symbol.clone());
        self.symbols.insert(symbol.text.clone(), id);
        self.pending.push(DocumentOp::CreateSymbol(id, symbol));
        id
    }
    pub fn add_dict(&mut self, dict: Dictionary) -> DictId {
        let id = self.storage.dicts.create(self.site, dict.clone());
        self.pending.push(DocumentOp::CreateDictionary(id, dict));
        id
    }
    pub fn load_dict(&mut self, data: &[u8]) -> DictId {
        use hyphenation::{Standard, Load};
        use std::io::Cursor;
        self.add_dict(
            Dictionary::Standard(Standard::any_from_reader(&mut Cursor::new(data)).unwrap())
        )
    }
    pub fn find_symbol(&self, text: &str) -> Option<SymbolId> {
        self.symbols.get(text).cloned()
    }

    fn link(&mut self, tag: Tag) {
        if let Some(Item::Sequence(child_id)) = self.storage.get_item(tag) {
            self.parents.insert(child_id, tag);
        }
    }
    fn unlink(&mut self, tag: Tag) {
        if let Some(Item::Sequence(child_id)) = self.storage.get_item(tag) {
            self.parents.remove(&child_id);
        }
    }
    pub fn get_previous_tag_bounded(&self, tag: Tag) -> Option<Tag> {
        let weave = self.storage.get_weave(tag.seq());
        let prev = match tag {
            Tag::Start(_) => return None,
            Tag::Item(_, id) => match weave.get_item(id).unwrap() {
                Item::Sequence(child) => return Some(Tag::End(child)),
                _ => weave.get_previous(id)
            }
            Tag::End(_) => weave.get_last(),
        };
        match prev {
            Some((id, _)) => Some(Tag::Item(tag.seq(), id)),
            None => Some(Tag::Start(tag.seq()))
        }
    }
    pub fn get_previous_tag(&self, tag: Tag) -> Option<Tag> {
        if let Some(tag) = self.get_previous_tag_bounded(tag) {
            return Some(tag);
        }

        // from here on things can fail, in which case ther is no previous tag

        // no parent -> we are at the root (or in an unlinked node…)
        let parent_tag = self.parents.get(&tag.seq())?;
        let parent_seq = parent_tag.seq();
        let parent = self.storage.get_weave(parent_seq);
        let parent_item_id = parent_tag.item().unwrap();
        match parent.get_previous(parent_item_id) {
            Some((id, _)) => return Some(Tag::Item(parent_seq, id)),
            None => return Some(Tag::Start(parent_seq))
        }
    }
    pub fn get_next_tag_bounded(&self, tag: Tag) -> Option<Tag> {
        let weave = self.storage.get_weave(tag.seq());
        let next = match tag {
            Tag::Start(_) => weave.get_first(),
            Tag::Item(_, id) => weave.get_next(id),
            Tag::End(_) => return None,
        };
        match next {
            Some((_, Item::Sequence(child))) => Some(Tag::Start(child)),
            Some((id, _)) => Some(Tag::Item(tag.seq(), id)),
            None => Some(Tag::End(tag.seq()))
        }
    }
    pub fn get_next_tag(&self, tag: Tag) -> Option<Tag> {
        if let Some(tag) = self.get_next_tag_bounded(tag) {
            return Some(tag);
        }

        // from here on things can fail, in which case there is no next tag

        // no parent -> we are at the root (or in an unlinked node…)
        let parent_tag = self.parents.get(&tag.seq())?;
        let parent_seq = parent_tag.seq();
        let parent = self.storage.get_weave(parent_seq);
        let parent_item_id = parent_tag.item().unwrap();
        match parent.get_next(parent_item_id) {
            Some((_, Item::Sequence(child))) => return Some(Tag::Start(child)),
            Some((id, _)) => return Some(Tag::Item(parent_seq, id)),
            None => return Some(Tag::Start(parent_seq))
        }
    }
    pub fn get_parent_tag(&self, tag: Tag) -> Option<Tag> {
        self.parents.get(&tag.seq()).cloned()
    }

    pub fn create_symbol(&mut self, text: &str) -> SymbolId {
        let id = self.find_symbol(text).unwrap_or_else(|| {
            // here we go…
            let mut chars = text.chars();
            let c = chars.next().expect("empty string");
            assert!(chars.next().is_none(), "only one char allowed");

            let leading = c.is_punctuation_initial_quote() | c.is_punctuation_open();
            let trailing = c.is_punctuation_final_quote() | c.is_punctuation_close();

            let overflow_left = c.is_punctuation_initial_quote();
            let overflow_right = c.is_punctuation_final_quote();

            self.add_symbol(Symbol {
                text: text.into(),
                leading,
                trailing,
                overflow_left: overflow_left as u8 as f32,
                overflow_right: overflow_right as u8 as f32,
            })
        });
        id
    }

    pub fn create_text<'a>(&'a mut self, text: &'a str) -> impl Iterator<Item=Item> + 'a {
        text.split_word_bounds().filter_map(move |part| {
            let first_char = part.chars().next().unwrap();
            if first_char.is_whitespace() {
                None // nothing to do
            }
            else if first_char.is_letter() | first_char.is_number() {
                Some(Item::Word(self.create_word(part)))
            } else {
                Some(Item::Symbol(self.create_symbol(part)))
            }
        })
    }
}