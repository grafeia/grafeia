use grafeia_core::*;
use docx::{Docx, document::{Para, Text, TextSpace}};
use std::io::Cursor;

struct DocxWriter<'a> {
    docx: Docx<'a>,
    run: Vec<Text<'a>>
}
impl<'a> DocxWriter<'a> {
    fn file_suffix() -> &'static str { "docx" }
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

pub fn export_docx(storage: &Storage, document: &Document, design: &Design) -> Vec<u8> {
    let mut writer = DocxWriter::new();
    for (_, r) in document.items(..) {
        match r {
            FindResult::SequenceStart(s) => {
                let type_design = design.get_type_or_default(s.typ());
                match type_design.display {
                    Display::Inline => {},
                    Display::Paragraph(_) | Display::Block => writer.flush_para(),
                }
            }
            FindResult::Item(_, &Item::Word(key)) => writer.word(&storage.get_word(key).text),
            _ => {}
        }
    }
    writer.flush_para();
    writer.finish()
}
