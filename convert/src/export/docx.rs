use grafeia_core::*;
use docx::{Docx, document::{Para, Text, TextSpace}};
use std::io::Cursor;

struct DocxWriter<'a> {
    docx: Docx<'a>,
    run: Vec<Text<'a>>
}
impl<'a> DocxWriter<'a> {
    fn new() -> Self {
        DocxWriter {
            docx: Docx::default(),
            run: vec![]
        }
    }
    fn flush_para(&mut self) {
        if self.run.len() == 0 {
            return;
        }
        let mut para = Para::default();
        for text in self.run.drain(..) {
            para.text(text);
        }
        self.docx.insert_para(para);
    }
    fn word(&mut self, word: &'a str) {
        if self.run.len() > 0 {
            self.run.push(Text::new(" ", Some(TextSpace::Preserve)));
        }
        self.run.push(Text::new(word, Some(TextSpace::Preserve)));
    }
    fn finish(mut self) -> Vec<u8> {
        let mut data = Vec::new();
        self.docx.write(Cursor::new(&mut data)).unwrap();
        data
    }
}

fn add_sequence<'a>(writer: &mut DocxWriter<'a>, storage: &'a Storage, key: SequenceId, design: &'a Design) {
    let seq = storage.get_weave(key);

    let type_design = design.get_type_or_default(seq.typ());
    match type_design.display {
        Display::Inline => {},
        Display::Paragraph(_, _) | Display::Block(_) => writer.flush_para(),
    }

    for item in seq.render() {
        match item {
            Item::Word(key) => writer.word(&storage.get_word(key).text),
            Item::Sequence(key) => add_sequence(writer, storage, key, design),
            _ => {}
        }
    }
}

pub fn export_docx(state: &State) -> Vec<u8> {
    let mut writer = DocxWriter::new();
    add_sequence(&mut writer, &state.storage, state.root, &state.design);
    writer.flush_para();
    writer.finish()
}
