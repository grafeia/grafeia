use pulldown_cmark::{Parser, Event, Tag, CodeBlockKind};
use grafeia_core::*;
use std::mem::replace;

pub fn markdown_design(document: &mut Document) -> Design {
    let coramont_regular = document.add_font(
        &include_bytes!("../../../data/Cormorant-Regular.ttf")[..]
    );
    let coramont_italic = document.add_font(
        &include_bytes!("../../../data/Cormorant-Regular.ttf")[..]
    );
    let didot = document.add_font(
        &include_bytes!("../../../data/GFSDidot.otf")[..]
    );
    
    let default = TypeDesign {
        display:        Display::Paragraph(Length::mm(0.0)),
        font:           Font {
            font_face: coramont_regular,
            size: Length::mm(5.0)
        },
        word_space: FlexMeasure {
            height:  Length::zero(),
            shrink:  Length::mm(2.0),
            width:   Length::mm(3.0),
            stretch: Length::mm(5.0)
        },
        line_height: Length::mm(6.0)
    };
    let mut design = Design::new("default design".into(), default);
    for (i, &size) in [15.0, 12.0, 10.0, 8.0, 6.0, 5.0f32].iter().enumerate() {
        let name = &format!("header_{}", i + 1);
        design.set_type(
            document.find_type(name).unwrap(),
            TypeDesign {
                display:        Display::Block,
                font:           Font {
                    font_face: didot,
                    size: Length::mm(size)
                },
                word_space: FlexMeasure {
                    height:  Length::zero(),
                    shrink:  Length::mm(0.2 * size),
                    width:   Length::mm(0.3 * size),
                    stretch: Length::mm(0.5 * size)
                },
                line_height: Length::mm(1.25 * size)
            }
        );
    }
    design.set_type(
        document.find_type("paragraph").unwrap(),
        TypeDesign {
            display:        Display::Paragraph(Length::mm(10.0)),
            font:           Font {
                font_face: coramont_regular,
                size: Length::mm(5.0)
            },
            word_space: FlexMeasure {
                height:  Length::zero(),
                shrink:  Length::mm(2.0),
                width:   Length::mm(3.0),
                stretch: Length::mm(5.0)
            },
            line_height: Length::mm(6.0)
        }
    );
    design.set_type(
        document.find_type("list").unwrap(),
        TypeDesign {
            display:        Display::Block,
            font:           Font {
                font_face: coramont_regular,
                size: Length::mm(5.0)
            },
            word_space: FlexMeasure {
                height:  Length::zero(),
                shrink:  Length::mm(2.0),
                width:   Length::mm(3.0),
                stretch: Length::mm(5.0)
            },
            line_height: Length::mm(6.0)
        }
    );
    design.set_type(
        document.find_type("emphasis").unwrap(),
        TypeDesign {
            display:        Display::Inline,
            font:           Font {
                font_face: coramont_italic,
                size: Length::mm(5.0)
            },
            word_space: FlexMeasure {
                height:  Length::zero(),
                shrink:  Length::mm(2.0),
                width:   Length::mm(3.0),
                stretch: Length::mm(5.0)
            },
            line_height: Length::mm(6.0)
        }
    );
    design
}

pub fn define_types(document: &mut Document) {
    for level in 1 ..= 6 {
        document.create_type(&format!("header_{}", level), Type::new(format!("Markdown Heading #{}", level)));
    }
    let mut add_type = |name: &str, description: &'static str| -> TypeId {
        document.create_type(name, Type::new(format!("Markdown {}", description)))
    };

    add_type("document", "Document");
    add_type("paragraph", "Paragraph");
    add_type("emphasis", "Emphasised text");
    add_type("blockquote", "Quotation in block form");
    add_type("inline-code", "Inline Code");
    add_type("block-code", "Code in block form");
    add_type("list", "Unnumbered list of items");
}

fn text_items<'a>(document: &'a mut Document, text: &'a str) -> impl Iterator<Item=Item> + 'a {
    text.split(char::is_whitespace).filter(|&s| s.len() > 0)
        .map(move |s| Item::Word(document.create_word(s)))
}

pub fn import_markdown(document: &mut Document, text: &str) -> SequenceId {
    let document_typ = document.find_type("document").unwrap();
    let paragraph = document.find_type("paragraph").unwrap();
    let emphasis = document.find_type("emphasis").unwrap();
    let block_quote = document.find_type("blockquote").unwrap();
    let inline_code = document.find_type("inline-code").unwrap();
    let list = document.find_type("list").unwrap();
    let _block_code = document.find_type("block-code").unwrap();
    let headings: Vec<TypeId> = (1 ..= 6)
        .map(|level| document.find_type(&format!("header_{}", level)).unwrap())
        .collect();

    let mut stack = vec![];
    let mut items = vec![];
    let mut current_key = document_typ;

    let mut events = Parser::new(text).into_iter();
    while let Some(event) = events.next() {
        dbg!(&event);
        match event {
            Event::Start(tag) => {
                let key = match tag {
                    Tag::Paragraph => paragraph,
                    Tag::Heading(level) => headings.get(level as usize).expect("invalid heading level").clone(),
                    Tag::BlockQuote => block_quote,
                    Tag::Emphasis => emphasis,
                    Tag::List(None) => list,
                    Tag::Item => {
                        items.extend(text_items(document, "·"));
                        paragraph
                    }
                    Tag::CodeBlock(lang) => {
                        let mut code = String::new();
                        while let Some(event) = events.next() {
                            match event {
                                Event::End(_) => break,
                                Event::Text(text) | Event::Code(text) => code.push_str(&text),
                                _ => unreachable!()
                            }
                        }
                        match lang {
                            CodeBlockKind::Fenced(s) => match s.as_ref() {
                                "tex" | "TeX" | "latex" | "LaTeX" => {
                                    let key = document.create_object(Object::TeX(TeX::display(code)));
                                    items.push(Item::Object(key));
                                },
                                _ => {}
                            }
                            CodeBlockKind::Indented => {}
                        }
                        continue;
                    }
                    _ => panic!("tag {:?} not implemented", tag)
                };
                stack.push((current_key, replace(&mut items, vec![])));
                current_key = key;
            }
            Event::End(_) => {
                let (parent_key, parent_items) = stack.pop().unwrap();
                let inner_items = replace(&mut items, parent_items);
                let id = document.creat_seq_with_items(current_key, inner_items);
                items.push(Item::Sequence(id));
                current_key = parent_key;
            }
            Event::Text(text) => {
                items.extend(text_items(document, text.as_ref()));
            }
            Event::Code(text) => {
                let text_items: Vec<_> = text_items(document, text.as_ref()).collect();
                let id = document.creat_seq_with_items(inline_code, text_items);
                items.push(Item::Sequence(id));
            }
            _ => {}
        }
    }

    document.creat_seq_with_items(document_typ, items)
}
