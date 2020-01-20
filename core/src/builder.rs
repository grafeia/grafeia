use crate::content::{Storage, Sequence, TypeKey, Item, Type};

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
                "paragraph".into(),
                Type { description: "A Paragraph".into() }
            ),
            chapter_key: storage.insert_type(
                "chapter".into(),
                Type { description: "A Chapter".into() }
            ),
            document_key: storage.insert_type(
                "document".into(),
                Type { description: "The Document".into() }
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
    pub fn finish(self) -> Item {
        let seq = Sequence::new(self.document_key, self.items);
        Item::Sequence(Box::new(seq))
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

    pub fn finish(mut self) -> ContentBuilder<'a> {
        let seq = Sequence::new(self.typ, self.nodes);
        self.parent.items.push(Item::Sequence(Box::new(seq)));
        self.parent
    }
}