use crate::{Storage, Sequence, TypeKey, Item, Type, Document, Object};
use std::rc::Rc;

pub struct ContentBuilder<'a> {
    storage: &'a mut Storage,
    para_key: TypeKey,
    chapter_key: TypeKey,
    document_key: TypeKey,

    items: Vec<Item>
}
impl<'a> ContentBuilder<'a> {
    pub fn new(storage: &'a mut Storage) -> ContentBuilder<'a> {
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
    pub fn chapter(self) -> TextBuilder<'a> {
        TextBuilder {
            typ: self.chapter_key,
            nodes: vec![],
            parent: self
        }
    }
    pub fn paragraph(self) -> TextBuilder<'a> {
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
    pub fn finish(mut self) -> Document {
        let seq = Sequence::new(self.document_key, self.items);
        let root = self.storage.insert_sequence(seq);
        Document::new(&self.storage, root)
    }
}

pub struct TextBuilder<'a> {
    parent: ContentBuilder<'a>,
    typ:    TypeKey,
    nodes:  Vec<Item>
}
impl<'a> TextBuilder<'a> {
    pub fn word(mut self, w: &str) -> Self {
        let word = self.parent.storage.insert_word(w);
        self.nodes.push(Item::Word(word));
        self
    }

    pub fn object(mut self, object: Object) -> Self {
        let key = self.parent.storage.insert_object(object);
        self.nodes.push(Item::Object(key));
        self
    }

    pub fn finish(mut self) -> ContentBuilder<'a> {
        let seq = Sequence::new(self.typ, self.nodes);
        let key = self.parent.storage.insert_sequence(seq);
        self.parent.items.push(Item::Sequence(key));
        self.parent
    }
}