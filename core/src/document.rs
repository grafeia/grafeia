use crate::*;
use std::collections::HashMap;
use std::ops::{RangeBounds, Deref, Add, Index};
use std::cmp::{PartialEq, PartialOrd, Ordering};
use std::hash::Hash;
use std::fmt;

#[derive(Serialize, Deserialize)]
#[derive(Hash, Eq, PartialEq, Copy, Clone, Default, PartialOrd, Ord, Debug)]
pub struct SiteId(pub u16);

#[derive(Serialize, Deserialize)]
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Copy, Clone)]
struct Id {
    clock: u32,
    site:  SiteId,
}
impl Id {
    const fn null() -> Id {
        Id { clock: 0, site: SiteId(0) }
    }
    fn is_null(&self) -> bool {
        *self == Id::null()
    }
}
impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.site.0, self.clock)
    }
}

#[derive(Serialize, Deserialize)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone)]
pub struct Atom {
    prev: Id,
    op: AtomOp,
    id: Id,
}

macro_rules! id {
    ($($name:ident),*) => ( $(
        #[derive(Serialize, Deserialize)]
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
        pub struct $name(SiteId, u32);

        impl From<(SiteId, u32)> for $name {
            fn from((site, n): (SiteId, u32)) -> Self {
                $name(site, n)
            }
        }
    )* )
}

id!(WordId, SymbolId, ObjectId, SequenceId, TypeId, FontId);

#[derive(Serialize, Deserialize)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum GlobalItem {
    Word(WordId),
    Symbol(SymbolId),
    Sequence(SequenceId),
    Object(ObjectId),
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum AtomOp {
    Remove,
    Replace(GlobalItem),
    Add(GlobalItem),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Weave {
    atoms: Vec<Atom>,
    clock: u32
}
impl Weave {
    pub fn new() -> Weave {
        Weave {
            atoms: Vec::new(),
            clock: 0
        }
    }
    fn from_items(site: SiteId, items: impl Iterator<Item=GlobalItem>) -> Weave {
        let mut prev = Id::null();
        let mut weave = Weave::new();
        for item in items {
            let atom = weave.create(site, prev, AtomOp::Add(item));
            prev = atom.id;
            weave.add(atom);
        }
        weave
    }
    fn create(&self, site: SiteId, prev: Id, op: AtomOp) -> Atom {
        Atom {
            prev,
            id: Id { site, clock: self.clock },
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
        self.clock = self.clock.max(atom.id.clock) + 1;
    }
    fn items<'s>(&'s self) -> impl Iterator<Item=(Id, GlobalItem)> + 's {
        use crate::gen::GenIter;

        let mut pending: Option<(Id, GlobalItem)> = None;
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
    fn render<'s>(&'s self) -> impl Iterator<Item=GlobalItem> + 's {
        self.items().map(|(id, item)| item)
    }
    fn find(&self, idx: usize) -> Option<Id> {
        self.items().nth(idx).map(|(id, item)| id)
    }
}

pub trait Inc {
    fn inc(self) -> Self;
}
impl Inc for u32 {
    fn inc(self) -> Self { self + 1 }
}
#[derive(Default)]
struct Counter<T>(T);
impl<T> Counter<T> where T: Default + Inc + Clone {
    pub fn new() -> Self {
        Counter(T::default())
    }
    pub fn next(&mut self) -> T {
        let t = self.0.clone().inc();
        std::mem::replace(&mut self.0, t)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DocumentOp {
    SeqOp(SequenceId, Atom),
    CreateSequence(SequenceId, TypeId),
    CreateWord(WordId, String)
}

#[derive(Serialize, Deserialize)]
pub struct LocalDocument {
    storage: Storage,
    root: SequenceKey,
    parents: HashMap<SequenceKey, SequenceKey>,
}
impl Deref for LocalDocument {
    type Target = Storage;
    fn deref(&self) -> &Storage {
        &self.storage
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct GloblDesign {
    default: TypeDesign,
    entries: HashMap<TypeId, TypeDesign>,
    name:    String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GlobalDocument {
    root:    SequenceId,
    weaves:  HashMap<SequenceId, (Weave, TypeId)>,
    words:   HashMap<WordId,     String>,
    symbols: HashMap<SymbolId,   String>,
    objects: HashMap<ObjectId,   Object>,
    types:   HashMap<TypeId,     (Type, String)>,
    fonts:   HashMap<FontId,     FontFace>,
    target:  Target,
    design:  GloblDesign
}
impl GlobalDocument {
    pub fn apply(&mut self, doc_op: DocumentOp) {
        match doc_op {
            DocumentOp::SeqOp(id, op) => {
                self.weaves.get_mut(&id).unwrap().0.add(op);
            }
            DocumentOp::CreateSequence(id, typ) => {
                self.weaves.insert(id, (Weave::new(), typ));
            }
            DocumentOp::CreateWord(id, word) => {
                self.words.insert(id, word);
            }
        }
    }
}
struct KeyMap<Key, Id> {
    forward: HashMap<Key, Id>,
    reverse: HashMap<Id, Key>,
    counter: Counter<u32>
}
impl<Key: Eq + Hash, Id: Eq + Hash> Default for KeyMap<Key, Id> {
    fn default() -> Self {
        KeyMap {
            forward: HashMap::new(),
            reverse: HashMap::new(),
            counter: Counter(0)
        }
    }
}
impl<Key: Eq + Hash + Copy, Id: Eq + Hash + Copy + From<(SiteId, u32)>> KeyMap<Key, Id> {
    pub fn insert(&mut self, key: Key, id: Id) {
        self.forward.entry(key).or_insert(id);
        self.reverse.entry(id).or_insert(key);
    }
    pub fn id_for_key(&self, key: Key) -> Option<Id> {
        self.forward.get(&key).cloned()
    }
    pub fn key_for_id(&self, id: Id) -> Option<Key> {
        self.reverse.get(&id).cloned()
    }
    pub fn add_local(&mut self, site: SiteId, key: Key) -> Id {
        let id = Id::from((site, self.counter.next()));
        self.forward.insert(key, id);
        self.reverse.insert(id, key);
        id
    }
}

#[derive(Default)]
struct Map {
    sequences: KeyMap<SequenceKey, SequenceId>,
    words: KeyMap<WordKey, WordId>,
    symbols: KeyMap<SymbolKey, SymbolId>,
    objects: KeyMap<ObjectKey, ObjectId>,
    types: KeyMap<TypeKey, TypeId>,
    fonts: KeyMap<FontFaceKey, FontId>,
}
impl Map {
    fn to_global(&self, item: Item) -> GlobalItem {
        match item {
            Item::Word(key) => GlobalItem::Word(self.words.id_for_key(key).unwrap()),
            Item::Symbol(key) => GlobalItem::Symbol(self.symbols.id_for_key(key).unwrap()),
            Item::Object(key) => GlobalItem::Object(self.objects.id_for_key(key).unwrap()),
            Item::Sequence(key) => GlobalItem::Sequence(self.sequences.id_for_key(key).unwrap()),
        }
    }
    fn to_local(&self, item: GlobalItem) -> Item {
        let g_item = match item {
            GlobalItem::Word(id) => self.words.key_for_id(id).map(Item::Word),
            GlobalItem::Symbol(id) => self.symbols.key_for_id(id).map(Item::Symbol),
            GlobalItem::Object(id) => self.objects.key_for_id(id).map(Item::Object),
            GlobalItem::Sequence(id) => self.sequences.key_for_id(id).map(Item::Sequence),
        };
        g_item.unwrap()
    }
    fn add_local(&mut self, site: SiteId, item: Item) -> GlobalItem {
        match item {
            Item::Word(key) => {
                GlobalItem::Word(self.words.add_local(site, key))
            }
            Item::Symbol(key) => {
                GlobalItem::Symbol(self.symbols.add_local(site, key))
            }
            Item::Object(key) => {
                GlobalItem::Object(self.objects.add_local(site, key))
            }
            Item::Sequence(key) => {
                GlobalItem::Sequence(self.sequences.add_local(site, key))
            }
        }
    }
}

pub struct Document {
    local: LocalDocument,
    weaves: HashMap<SequenceId, Weave>,
    site: SiteId,
    pending: Vec<DocumentOp>,
    map: Map
}

fn walk(storage: &Storage, parent: SequenceKey, f: &mut impl FnMut(SequenceKey, SequenceKey)) {
    for item in storage.get_sequence(parent).items() {
        if let Item::Sequence(child) = *item {
            f(parent, child);
            walk(storage, child, f);
        }
    }
}

impl LocalDocument {
    pub fn new(storage: Storage, root: SequenceKey) -> LocalDocument {
        // set the parent fields of all sequences
        let mut parents = HashMap::new();
        walk(&storage, root, &mut |parent, child| {
            parents.insert(child, parent);
        });
        
        LocalDocument {
            storage,
            root,
            parents,
        }
    }
    pub fn root(&self) -> SequenceKey {
        self.root
    }
    fn link(&mut self, tag: Tag) {
        if let SequencePos::At(idx) = tag.pos {
            let seq = self.storage.get_sequence(tag.seq);
            if let Item::Sequence(child) = seq.items[idx] {
                if self.parents.insert(child, tag.seq) != Some(tag.seq) {
                    let parents = &mut self.parents;
                    walk(&self.storage, child, &mut |parent, child| {
                        parents.insert(child, parent);
                    });
                }
            }
        }
    }
    fn unlink(&mut self, tag: Tag) {
        if let SequencePos::At(idx) = tag.pos {
            let seq = self.storage.get_sequence(tag.seq);
            if let Item::Sequence(child) = seq.items[idx] {
                self.parents.remove(&child);
            }
        }
    }
    pub fn get_last(&self, key: SequenceKey) -> Tag {
        let seq = self.storage.get_sequence(key);
        let num_childs = seq.items.len();
        if num_childs == 0 {
            return Tag::end(key);
        }
        match seq.items[num_childs - 1] {
            Item::Sequence(child_key) => self.get_last(child_key),
            _ => Tag::at(key, num_childs - 1)
        }
    }
    pub fn get_first(&self, key: SequenceKey) -> Tag {
        let seq = self.storage.get_sequence(key);
        let num_childs = seq.items().len();
        if num_childs == 0 {
            return Tag::end(key);
        }
        match seq.items[0] {
            Item::Sequence(child_key) => self.get_first(child_key),
            _ => Tag::at(key, 0)
        }
    }

    pub fn get_previous_tag(&self, tag: Tag) -> Option<Tag> {
        let seq = self.storage.get_sequence(tag.seq);
        let idx = match tag.pos {
            SequencePos::At(idx) => idx,
            SequencePos::End => seq.items.len()
        };

        if idx > 0 {
            match seq.items[idx - 1] {
                Item::Sequence(child_key) => return Some(self.get_last(child_key)),
                _ => return Some(Tag::at(tag.seq, idx - 1))
            }
        }

        let &parent_key = self.parents.get(&tag.seq)?;
        let parent = self.storage.get_sequence(parent_key);
        let pos = parent.items.iter().position(|i| *i == Item::Sequence(tag.seq))?;
        Some(Tag::at(parent_key, pos))
    }
    pub fn get_next_tag(&self, tag: Tag) -> Option<Tag> {
        let seq = self.storage.get_sequence(tag.seq);
        if let SequencePos::At(idx) = tag.pos {
            if let Item::Sequence(key) = seq.items[idx] {
                let child_seq = self.storage.get_sequence(key);
                if child_seq.items.len() > 0 {
                    return Some(Tag::at(key, 0));
                } else {
                    return Some(Tag::end(key));
                }
            }
            if idx + 1 < seq.items.len() {
                return Some(Tag::at(tag.seq, idx + 1));
            } else {
                return Some(Tag::end(tag.seq));
            }
        }

        let &parent_key = self.parents.get(&tag.seq)?;
        let parent = self.storage.get_sequence(parent_key);
        let pos = parent.items.iter().position(|i| *i == Item::Sequence(tag.seq))?;
        if pos + 1 < parent.items.len() {
            Some(Tag::at(parent_key, pos + 1))
        } else {
            Some(Tag::end(parent_key))
        }
    }
    pub fn get_parent_tag(&self, tag: Tag) -> Option<Tag> {
        let &parent_key = self.parents.get(&tag.seq)?;
        let parent = self.storage.get_sequence(parent_key);
        let pos = parent.items.iter().position(|i| *i == Item::Sequence(tag.seq))?;
        Some(Tag::at(parent_key, pos))
    }
    pub fn childen<'s>(&'s self, parent: SequenceKey) -> impl Iterator<Item=Tag> + 's {
        use crate::gen::GenIter;
        let storage = &self.storage;
        GenIter::new(move || {
            let mut stack = vec![];
            let traverse = move |key| (key, storage.get_sequence(parent).items.iter().enumerate());

            let mut current = traverse(parent);

            loop {
                while let Some((idx, item)) = current.1.next() {
                    if let Item::Sequence(child) = *item {
                        stack.push(std::mem::replace(&mut current, traverse(child)));
                        continue;
                    } else {
                        yield Tag::at(current.0, idx);
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

    pub fn add_font(&mut self, data: impl Into<Vec<u8>>) -> FontFaceKey {
        self.storage.insert_font_face(FontFace::from_data(data.into()))
    }
}
impl Deref for Document {
    type Target = LocalDocument;
    fn deref(&self) -> &LocalDocument {
        &self.local
    }
}


impl Document {
    pub fn local(&self) -> &LocalDocument {
        &self.local
    }
    pub fn from_local(local: LocalDocument, site: SiteId) -> Document {
        let mut map = Map::default();

        for (key, _) in local.storage.words() {
            map.words.add_local(site, key);
        }
        for (key, _) in local.storage.symbols() {
            map.symbols.add_local(site, key);
        }
        for (key, _) in local.storage.objects() {
            map.objects.add_local(site, key);
        }
        for (key, _, _) in local.storage.types() {
            map.types.add_local(site, key);
        }
        for (key, _) in local.storage.fonts() {
            map.fonts.add_local(site, key);
        }
        for (key, _) in local.storage.sequences() {
            map.sequences.add_local(site, key);
        }

        let mut weaves = HashMap::new();
        let mut add_seq = |seq: SequenceKey| {
            let id = map.sequences.id_for_key(seq).unwrap();
            let weave = Weave::from_items(site,
                local.storage.get_sequence(seq).items.iter()
                .map(|&item| map.to_global(item))
            );
            weaves.insert(id, weave);
        };

        add_seq(local.root);
        walk(&local.storage, local.root, &mut |parent, child| add_seq(child));

        Document {
            site,
            map,
            weaves,
            local,
            pending: Vec::new()
        }
    }
    pub fn from_global(global: GlobalDocument, site: SiteId) -> (Document, Target, Design) {
        let mut storage = Storage::new();
        let mut map = Map::default();
        for (id, (typ, name)) in global.types {
            map.types.insert(storage.insert_type(name, typ), id);
        }
        for (id, word) in global.words {
            map.words.insert(storage.insert_word(&word), id);
        }
        for (id, symbol) in global.symbols {
            map.symbols.insert(storage.insert_symbol(&symbol), id);
        }
        for (id, object) in global.objects {
            map.objects.insert(storage.insert_object(object.clone()), id);
        }
        for (id, font) in global.fonts {
            map.fonts.insert(storage.insert_font_face(font.clone()), id);
        }
        let mut weaves = HashMap::new();
        for (id, (weave, typ_id)) in global.weaves {
            info!("weave {:?} {:?}", id, weave);
            let typ = map.types.key_for_id(typ_id).unwrap();
            map.sequences.insert(storage.insert_sequence(Sequence::new(typ, vec![])), id);
            weaves.insert(id, weave);
        }
        for (&id, weave) in weaves.iter() {
            let key = map.sequences.key_for_id(id).unwrap();
            let seq = storage.get_sequence_mut(key);
            seq.items.extend(
                weave.render().map(|item| map.to_local(item))
            );
            info!("weave: {:#?}", weave);
            info!("-> sequence: {:?}", seq);
        }

        let mut design = Design::new(global.design.name, global.design.default);
        for (id, typ) in global.design.entries {
            let key = map.types.key_for_id(id).unwrap();
            design.set_type(key, typ);
        }

        let root = map.sequences.key_for_id(global.root).unwrap();
        let local = LocalDocument::new(storage, root);

        let document = Document {
            site,
            local,
            map,
            pending: Vec::new(),
            weaves
        };

        (document, global.target, design)
    }
    pub fn to_global(&self, target: &Target, design: &Design) -> GlobalDocument {
        let map = &self.map;
        let local = &self.local;
        let weaves = self.weaves.iter().map(|(&id, weave)| {
            let seq_key = map.sequences.key_for_id(id).unwrap();
            let typ_key = self.storage.get_sequence(seq_key).typ();
            let typ_id = map.types.id_for_key(typ_key).unwrap();
            (id, (weave.clone(), typ_id))
        }).collect();

        GlobalDocument {
            root: map.sequences.id_for_key(local.root).unwrap(),
            weaves,
            words:   local.storage.words().map(|(key, word)|
                (map.words.id_for_key(key).unwrap(), word.text.clone())
            ).collect(),
            symbols: local.storage.symbols().map(|(key, symbol)|
                (map.symbols.id_for_key(key).unwrap(), symbol.text.clone())
            ).collect(),
            objects: local.storage.objects().map(|(key, object)|
                (map.objects.id_for_key(key).unwrap(), object.clone())
            ).collect(),
            types:   local.storage.types().map(|(key, name, typ)|
                (map.types.id_for_key(key).unwrap(), (typ.clone(), name.to_owned()))
            ).collect(),
            fonts:   local.storage.fonts().map(|(key, font)|
                (map.fonts.id_for_key(key).unwrap(), font.clone())
            ).collect(),
            target:  target.clone(),
            design: GloblDesign {
                name:    design.name().to_owned(),
                entries: design.items().map(|(key, typ)|
                    (map.types.id_for_key(key).unwrap(), typ.clone())
                ).collect(),
                default: design.default().clone(),
            }
        }
    }
    fn add_pending(&mut self, seq_key: SequenceKey, atom: Atom) {
        let id = self.map.sequences.id_for_key(seq_key).unwrap();
        self.pending.push(DocumentOp::SeqOp(id, atom));
    }
    pub fn drain_pending<'s>(&'s mut self) -> impl Iterator<Item=DocumentOp> + 's {
        self.pending.drain(..)
    }

    pub fn exec_op(&mut self, op: DocumentOp) {
        match op {
            DocumentOp::SeqOp(seq_id, atom) => {
                let weave = self.weaves.get_mut(&seq_id).unwrap();
                weave.add(atom);
                let map = &self.map;
                let seq_key = self.map.sequences.key_for_id(seq_id).unwrap();
                let seq = self.local.storage.get_sequence_mut(seq_key);
                seq.items.clear();
                seq.items.extend(
                    weave.render().map(|item| map.to_local(item))
                );
            }
            DocumentOp::CreateSequence(seq_id, typ_id) => {
                let typ_key = self.map.types.key_for_id(typ_id).unwrap();
                let seq = Sequence::new(typ_key, Vec::new());
                let seq_key = self.local.storage.insert_sequence(seq);

                self.map.sequences.insert(seq_key, seq_id);
                self.weaves.insert(seq_id, Weave::new());
            }
            DocumentOp::CreateWord(word_id, text) => {
                let word_key = self.local.storage.insert_word(&text);
                self.map.words.insert(word_key, word_id);
            }
        }
    }

    fn log_weave(&self, id: SequenceId) {
        let weave = self.weaves.get(&id).unwrap();
        let item = |item: GlobalItem| {
            match self.map.to_local(item) {
                Item::Word(key) => self.local.storage.get_word(key).text.as_str(),
                Item::Symbol(key) => self.local.storage.get_symbol(key).text.as_str(),
                Item::Sequence(_) => "<seq>",
                Item::Object(_) => "<obj>",
            }
        };

        for atom in weave.atoms.iter() {
            match atom.op {
                AtomOp::Remove => info!("{} Remove {}", atom.id, atom.prev),
                AtomOp::Replace(id) => info!("{} Replace({}) {}", atom.id, item(id), atom.prev),
                AtomOp::Add(id) => info!("{} Add({}) {}", atom.id, item(id), atom.prev),
            }
        }
    }

    pub fn replace(&mut self, tag: Tag, new_item: Item) {
        let (seq_key, idx) = tag.seq_and_idx().unwrap();
        self.local.unlink(tag);
        let seq = self.local.storage.get_sequence_mut(seq_key);

        let seq_id = self.map.sequences.id_for_key(seq_key).unwrap();
        let weave = self.weaves.get_mut(&seq_id).unwrap();
        let prev_id = weave.find(idx).unwrap();
        let atom = weave.create(self.site, prev_id, AtomOp::Replace(self.map.to_global(new_item)));
        weave.add(atom);
        let map = &self.map;
        seq.items.clear();
        seq.items.extend(
            weave.render().map(|item| map.to_local(item))
        );
        info!("replace -> {:?}", seq.items);
        self.log_weave(seq_id);
        self.pending.push(DocumentOp::SeqOp(seq_id, atom));

        self.local.link(tag);
    }

    // returns the tag where, if the item was inserted again, the original state would be restored
    pub fn remove(&mut self, tag: Tag) -> (Tag, Item) {
        let (seq_key, idx) = tag.seq_and_idx().unwrap();
        self.local.unlink(tag);
        let seq = self.local.storage.get_sequence_mut(seq_key);
        let item = seq.items[idx];

        let seq_id = self.map.sequences.id_for_key(seq_key).unwrap();
        let weave = self.weaves.get_mut(&seq_id).unwrap();
        let prev_id = weave.find(idx).unwrap();
        let atom = weave.create(self.site, prev_id, AtomOp::Remove);
        let map = &self.map;
        weave.add(atom);
        seq.items.clear();
        seq.items.extend(
            weave.render().map(|item| map.to_local(item))
        );

        let new_pos = if idx >= seq.items.len() {
            SequencePos::End
        } else {
            SequencePos::At(idx)
        };

        info!("remove -> {:?}", seq.items);
        self.log_weave(seq_id);
        self.pending.push(DocumentOp::SeqOp(seq_id, atom));
        (Tag { seq: seq_key, pos: new_pos }, item)
    }

    pub fn insert(&mut self, tag: Tag, new_item: Item) {
        let seq_key = tag.seq;
        let seq = self.local.storage.get_sequence_mut(seq_key);
        let idx = match tag.pos {
            SequencePos::At(idx) => idx,
            SequencePos::End => seq.items.len()
        };

        info!("insert {:?} at {}: {:?}", new_item, idx, seq.items);
        let seq_id = self.map.sequences.id_for_key(seq_key).unwrap();
        let weave = self.weaves.get_mut(&seq_id).unwrap();
        let prev_id = weave.find(idx).unwrap_or(Id::null());
        info!("prev id {}", prev_id);
        let atom = weave.create(self.site, prev_id, AtomOp::Add(self.map.to_global(new_item)));
        let map = &self.map;
        weave.add(atom);
        seq.items.clear();
        seq.items.extend(
            weave.render().map(|item| map.to_local(item))
        );
        info!("insert -> {:?}", seq.items);
        self.log_weave(seq_id);
        self.pending.push(DocumentOp::SeqOp(seq_id, atom));

        self.local.link(tag);
    }

    pub fn add_word(&mut self, text: &str) -> Item {
        let key = self.local.storage.insert_word(text);
        if !self.map.words.forward.contains_key(&key) {
            let id = self.map.words.add_local(self.site, key);
            self.pending.push(DocumentOp::CreateWord(id, text.to_owned()));
        }
        Item::Word(key)
    }

    pub fn crate_seq(&mut self, typ: TypeKey) -> Item {
        let key = self.local.storage.insert_sequence(Sequence::new(typ, vec![]));
        let id = self.map.sequences.add_local(self.site, key);
        let typ = self.map.types.id_for_key(typ).unwrap();
        self.pending.push(DocumentOp::CreateSequence(id, typ));
        Item::Sequence(key)
    }
}