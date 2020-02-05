use slotmap::SlotMap;
use indexmap::{IndexMap, IndexSet};
use serde::{Serialize, Deserialize};
use crate::content::{Word, Symbol, Type, FontFace, Target, Sequence, Item};
use crate::object::Object;

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
    pub struct ObjectKey;
}

#[derive(Serialize, Deserialize)]
pub struct Storage {
    words:   IndexSet<Word>,
    symbols: IndexSet<Symbol>,
    types:   IndexMap<String, Type>,
    fonts:   SlotMap<FontFaceKey, FontFace>,
    objects: SlotMap<ObjectKey, Object>
}

impl Storage {
    pub fn new() -> Storage {
        Storage {
            words:   IndexSet::new(),
            symbols: IndexSet::new(),
            types:   IndexMap::new(),
            fonts:   SlotMap::with_key(),
            objects: SlotMap::with_key(),
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
    pub fn insert_type(&mut self, key: impl Into<String>, typ: Type) -> TypeKey {
        let (idx, _) = self.types.insert_full(key.into(), typ);
        TypeKey(idx)
    }
    pub fn insert_font_face(&mut self, font_face: FontFace) -> FontFaceKey {
        self.fonts.insert(font_face)
    }
    pub fn insert_object(&mut self, obj: Object) -> ObjectKey {
        self.objects.insert(obj)
    }

    pub fn get_word(&self, key: WordKey) -> &Word {
        self.words.get_index(key.0).unwrap()
    }
    pub fn get_symbol(&self, key: SymbolKey) -> &Symbol {
        self.symbols.get_index(key.0).unwrap()
    }
    pub fn get_object(&self, key: ObjectKey) -> &Object {
        self.objects.get(key).unwrap()
    }
    pub fn get_font_face(&self, key: FontFaceKey) -> &FontFace {
        self.fonts.get(key).unwrap()
    }
    pub fn find_type(&self, name: &str) -> Option<TypeKey> {
        self.types.get_full(name).map(|(idx, _, _)| TypeKey(idx))
    }
    pub fn get_type(&self, key: TypeKey) -> (&str, &Type) {
        let (name, typ) = self.types.get_index(key.0).unwrap();
        (name.as_str(), typ)
    }
}
