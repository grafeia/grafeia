use grafeia_convert::build;
use grafeia_core::*;
use std::borrow::Cow;
use std::fs::{self, File};
use std::collections::HashMap;

use pulldown_cmark::{Parser, Event, Tag, CodeBlockKind};
use grafeia_core::*;
use grafeia_core::object::tex::TeX;
use grafeia_convert::build::DICT_EN_GB;
use std::mem::replace;
use std::io::BufWriter;

macro_rules! font {
    ($name:tt) => (
        &include_bytes!(concat!("../../../data/", $name))[..]
    )
}

fn skip(events: &mut Parser) {
    let mut n = 0;
    for e in events {
        match e {
            Event::Start(_) => n += 1,
            Event::End(_) if n == 0 => return,
            Event::End(_) => n -= 1,
            _ => {}
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let output = File::create(args.next().expect("no output file given")).expect("can't create output file");

    let storage = Storage::new();
    let mut document = Document::new(storage);
    build::symbols(&mut document);
    
    let headings: Vec<TypeId> = (1 ..= 6).map(|level|
        document.create_type(&format!("header_{}", level), Type::new(format!("Markdown Heading #{}", level)))
    ).collect();

    let mut add_type = |name: &str, description: &'static str| -> TypeId {
        document.create_type(name, Type::new(format!("Markdown {}", description)))
    };

    let document_typ = add_type("document", "Document");
    let paragraph = add_type("paragraph", "Paragraph");
    let emphasis = add_type("emphasis", "Emphasised text");
    let strong = add_type("strong", "Strong text?");
    let block_quote = add_type("blockquote", "Quotation in block form");
    let inline_code = add_type("inline-code", "Inline Code");
    let block_code = add_type("block-code", "Code in block form");
    let list = add_type("list", "Unnumbered list of items");
    let list_item = add_type("list-item", "A list items");

    let coramont_regular = document.add_font(font!("Cormorant-Regular.ttf"));
    let coramont_bold = document.add_font(font!("Cormorant-Bold.ttf"));
    let coramont_italic = document.add_font(font!("Cormorant-Italic.ttf"));
    let didot = document.add_font(font!("GFSDidot.otf"));
    let cutive_mono = document.add_font(font!("CutiveMono-Regular.ttf"));
    let latinmodern_math = document.add_font(font!("latinmodern-math.otf"));

    
    let hyphen = document.add_symbol(Symbol {
        text: "‐".into(),
        leading: false,
        trailing: true,
        overflow_left: 0.0,
        overflow_right: 1.0
    });

    let bullet = document.add_symbol(Symbol {
        text: "·".into(),
        leading: true,
        trailing: false,
        overflow_left: 1.0,
        overflow_right: 0.0
    });

    let dictionary = document.load_dict(DICT_EN_GB);

    let default = TypeDesign {
        display:        Display::Paragraph(
            Length::mm(0.0),
            VerticalPadding {
                above: Length::zero(),
                below: Length::mm(4.0)
            }
        ),
        font:           Font {
            font_face: coramont_regular,
            size: Length::mm(4.0)
        },
        word_space: FlexMeasure {
            shrink:  Length::mm(1.0),
            length:  Length::mm(1.2),
            stretch: Length::mm(2.0)
        },
        line_height: Length::mm(5.0),
        indent: Length::zero(),
        dictionary,
        hyphen: Some(hyphen),
    };
    let mut design = Design::new("default design".into(), default.clone());
    for (&typ, &size) in headings.iter().zip([10.0, 8.0, 6.0, 5.0, 5.0, 5.0f32].iter()) {
        design.set_type(typ,
            TypeDesign {
                display:        Display::Block(
                    VerticalPadding {
                        above: Length::mm(0.72 * size),
                        below: Length::mm(0.25 * size)
                    }
                ),
                font:           Font {
                    font_face: didot,
                    size: Length::mm(size)
                },
                word_space: FlexMeasure {
                    shrink:  Length::mm(0.2 * size),
                    length:  Length::mm(0.3 * size),
                    stretch: Length::mm(0.5 * size)
                },
                line_height: Length::mm(1.25 * size),
                indent: Length::zero(),
                dictionary,
                hyphen: None,
            }
        );
    }
    design.set_type(
        paragraph,
        TypeDesign {
            display:        Display::Paragraph(
                Length::mm(5.0),
                VerticalPadding {
                    above: Length::zero(),
                    below: Length::mm(4.0)
                }
            ),
            .. default
        }
    );
    design.set_type(
        block_code,
        TypeDesign {
            display:        Display::Paragraph(
                Length::mm(5.0),
                VerticalPadding {
                    above: Length::zero(),
                    below: Length::mm(4.0)
                }
            ),
            indent:         Length::mm(10.),
            .. default
        }
    );
    design.set_type(
        list,
        TypeDesign {
            display:        Display::Block(
                VerticalPadding {
                    above: Length::zero(),
                    below: Length::mm(4.0)
                }
            ),
            indent:         Length::mm(5.),
            .. default
        }
    );
    design.set_type(
        list_item,
        TypeDesign {
            display:        Display::Paragraph(
                Length::mm(0.0),
                VerticalPadding {
                    above: Length::zero(),
                    below: Length::zero()
                }
            ),
            .. default
        }
    );
    design.set_type(
        emphasis,
        TypeDesign {
            display:        Display::Inline,
            font:           Font {
                font_face: coramont_italic,
                size: Length::mm(4.0)
            },
            .. default
        }
    );
    design.set_type(
        strong,
        TypeDesign {
            display:        Display::Inline,
            font:           Font {
                font_face: coramont_bold,
                size: Length::mm(4.0)
            },
            .. default
        }
    );
    design.set_type(
        inline_code,
        TypeDesign {
            display:        Display::Inline,
            font:           Font {
                font_face:  cutive_mono,
                size: Length::mm(3.8)
            },
            hyphen: None,
            .. default
        }
    );

    let mut stack = vec![];
    let mut items = vec![];
    let mut current_key = document_typ;

    let mut list_numbers = HashMap::new();

    for path in args {
        let data = fs::read(path).unwrap();
        let mut list_nr = None;
        let mut events = Parser::new(std::str::from_utf8(&data).unwrap()).into_iter();
        while let Some(event) = events.next() {
            println!("{:?}", event);
            match event {
                Event::Start(tag) => {
                    let mut inner_items = vec![];
                    let key = match tag {
                        Tag::Paragraph => paragraph,
                        Tag::Heading(level) => headings.get(level as usize).expect("invalid heading level").clone(),
                        Tag::BlockQuote => block_quote,
                        Tag::Emphasis => emphasis,
                        Tag::List(first_nr) => {
                            list_nr = first_nr;
                            list
                        }
                        Tag::Item => {
                            let item = match list_nr {
                                None => bullet,
                                Some(n) => {
                                    list_nr = Some(n + 1);
                                    *list_numbers.entry(n).or_insert_with(|| {
                                        document.add_symbol(Symbol {
                                            text: format!("{}.", n),
                                            leading: true,
                                            trailing: false,
                                            overflow_left: 1.0,
                                            overflow_right: 0.0
                                        })
                                    })
                                }
                            };
                            inner_items.push(Item::Symbol(item));
                            list_item
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
                                        let key = document.create_object(Object::TeX(TeX::display(code, latinmodern_math)));
                                        items.push(Item::Object(key));
                                    },
                                    _ => {}
                                }
                                CodeBlockKind::Indented => {}
                            }
                            continue;
                        }
                        Tag::Strong => strong,
                        _ => {
                            skip(&mut events);
                            continue;
                        }
                    };
                    stack.push((current_key, replace(&mut items, inner_items)));
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
                    items.extend(document.create_text(text.as_ref()));
                }
                Event::Code(text) => {
                    let words: Vec<Item> = text.split_whitespace().map(|word| Item::Word(document.create_word(word))).collect();

                    let id = document.creat_seq_with_items(inline_code, words);
                    items.push(Item::Sequence(id));
                }
                _ => {}
            }
        }
        assert_eq!(stack.len(), 0);
    }

    let root = document.creat_seq_with_items(document_typ, items);
    document.set_root(root);

    let storage = document.into_storage();
    let target = build::default_target();

    let state = State {
        storage: Cow::Owned(storage),
        target: Cow::Owned(target),
        design: Cow::Owned(design),
        root
    };
    state.store(BufWriter::new(output)).unwrap();
}
