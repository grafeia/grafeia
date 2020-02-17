use crate::{Storage, Sequence, TypeKey, Item, Type, LocalDocument, Object};

pub struct ContentBuilder {
    storage: Storage,
    para_key: TypeKey,
    chapter_key: TypeKey,
    document_key: TypeKey,

    items: Vec<Item>
}
impl ContentBuilder {
    pub fn new() -> Self {
        let mut storage = Storage::new();
        ContentBuilder {
            para_key: storage.insert_type(
                "paragraph",
                Type::new("A Paragraph")
            ),
            chapter_key: storage.insert_type(
                "chapter",
                Type::new("A Chapter")
            ),
            document_key: storage.insert_type(
                "document",
                Type::new("The Document")
            ),
            storage,
            items: vec![]
        }
    }
    pub fn chapter(self) -> TextBuilder {
        TextBuilder {
            typ: self.chapter_key,
            nodes: vec![],
            parent: self
        }
    }
    pub fn paragraph(self) -> TextBuilder {
        TextBuilder {
            typ: self.para_key,
            nodes: vec![],
            parent: self
        }
    }
    pub fn object(mut self, object: Object) -> Self {
        let key = self.storage.insert_object(object);
        self.items.push(Item::Object(key));
        self
    }
    pub fn finish(mut self) -> LocalDocument {
        let seq = Sequence::new(self.document_key, self.items);
        let root = self.storage.insert_sequence(seq);
        LocalDocument::new(self.storage, root)
    }
}

pub struct TextBuilder {
    parent: ContentBuilder,
    typ:    TypeKey,
    nodes:  Vec<Item>
}
impl TextBuilder {
    pub fn word(mut self, w: &str) -> Self {
        let word = self.parent.storage.insert_word(w);
        self.nodes.push(Item::Word(word));
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        for w in text.split_ascii_whitespace() {
            let word = self.parent.storage.insert_word(w);
            self.nodes.push(Item::Word(word));
        }
        self
    }

    pub fn object(mut self, object: Object) -> Self {
        let key = self.parent.storage.insert_object(object);
        self.nodes.push(Item::Object(key));
        self
    }

    pub fn finish(mut self) -> ContentBuilder {
        let seq = Sequence::new(self.typ, self.nodes);
        let key = self.parent.storage.insert_sequence(seq);
        self.parent.items.push(Item::Sequence(key));
        self.parent
    }
}