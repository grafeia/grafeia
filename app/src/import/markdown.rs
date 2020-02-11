use pulldown_cmark::{Parser, Event, Tag};
use grafeia_core::*;
use std::mem::replace;

pub fn markdown_design(storage: &mut Storage) -> Design {
    let coramont_regular = storage.insert_font_face(
        Vec::from(&include_bytes!("../../../data/Cormorant-Regular.ttf")[..]).into()
    );
    let coramont_italic = storage.insert_font_face(
        Vec::from(&include_bytes!("../../../data/Cormorant-Regular.ttf")[..]).into()
    );
    let didot = storage.insert_font_face(
        Vec::from(&include_bytes!("../../../data/GFSDidot.otf")[..]).into()
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
            storage.find_type(name).unwrap(),
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
        storage.find_type("paragraph").unwrap(),
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
        storage.find_type("list").unwrap(),
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
        storage.find_type("emphasis").unwrap(),
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

pub fn define_types(storage: &mut Storage) {
    for level in 1 ..= 6 {
        storage.insert_type(&format!("header_{}", level), Type::new(format!("Markdown Heading #{}", level)));
    }
    let mut add_type = |name: &'static str, description: &'static str| -> TypeKey {
        storage.insert_type(name, Type::new(format!("Markdown {}", description)))
    };

    add_type("document", "Document");
    add_type("paragraph", "Paragraph");
    add_type("emphasis", "Emphasised text");
    add_type("blockquote", "Quotation in block form");
    add_type("inline-code", "Inline Code");
    add_type("block-code", "Code in block form");
    add_type("list", "Unnumbered list of items");
}

fn text_items<'a>(storage: &'a mut Storage, text: &'a str) -> impl Iterator<Item=Item> + 'a {
    text.split(char::is_whitespace).filter(|&s| s.len() > 0)
        .map(move |s| Item::Word(storage.insert_word(s)))
}

pub fn import_markdown(storage: &mut Storage, text: &str) -> Document {
    let document = storage.find_type("document").unwrap();
    let paragraph = storage.find_type("paragraph").unwrap();
    let emphasis = storage.find_type("emphasis").unwrap();
    let block_quote = storage.find_type("blockquote").unwrap();
    let inline_code = storage.find_type("inline-code").unwrap();
    let list = storage.find_type("list").unwrap();
    let block_code = storage.find_type("block-code").unwrap();
    let headings: Vec<TypeKey> = (1 ..= 6)
        .map(|level| storage.find_type(&format!("header_{}", level)).unwrap())
        .collect();

    let mut stack = vec![];
    let mut items = vec![];
    let mut current_key = document;

    for event in Parser::new(text) {
        dbg!(&event);
        match event {
            Event::Start(tag) => {
                stack.push((current_key, replace(&mut items, vec![])));

                let key = match tag {
                    Tag::Paragraph => paragraph,
                    Tag::Heading(level) => headings.get(level as usize).expect("invalid heading level").clone(),
                    Tag::BlockQuote => block_quote,
                    Tag::Emphasis => emphasis,
                    Tag::List(None) => list,
                    Tag::Item => {
                        items.extend(text_items(storage, "·"));
                        paragraph
                    }
                    _ => panic!("tag {:?} not implemented", tag)
                };
                current_key = key;
            }
            Event::End(_) => {
                let (parent_key, parent_items) = stack.pop().unwrap();
                let inner_items = replace(&mut items, parent_items);
                let seq = Sequence::new(current_key, inner_items);
                let key = storage.insert_sequence(seq);
                items.push(Item::Sequence(key));
                current_key = parent_key;
            }
            Event::Text(text) => {
                items.extend(text_items(storage, text.as_ref()));
            }
            Event::Code(text) => {
                let seq = Sequence::new(inline_code, text_items(storage, text.as_ref()).collect());
                let key = storage.insert_sequence(seq);
                items.push(Item::Sequence(key));
            }
            _ => {}
        }
    }

    let key = storage.insert_sequence(Sequence::new(document, items));
    Document::new(&storage, key)
}
