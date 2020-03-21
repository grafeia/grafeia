use grafeia_core::*;
use grafeia_core::builder::*;
use grafeia_core::object::{tex::*, svg::*};
use std::borrow::Cow;

pub static DICT_EN_GB: &'static [u8] = &*include_bytes!("../../data/dictionaries/en-gb.standard.bincode");

pub fn symbols(document: &mut Document) {
    let trailing = [",", ".", ":", "!", "?", ";", ];
    let quotes = [
        ("“", "”"),
        ("‘", "’"),
        ("«", "»"),
        ("‹", "›"),
    ];

    for &symbol in trailing.iter() {
        document.add_symbol(Symbol {
            text: symbol.to_owned(),
            trailing: true,
            leading: false,
            overflow_left: 0.0,
            overflow_right: 1.0,
        });
    }
    for &(open, close) in quotes.iter() {
        document.add_symbol(Symbol {
            text: open.to_owned(),
            trailing: false,
            leading: true,
            overflow_left: 1.0,
            overflow_right: 0.0,
        });
        document.add_symbol(Symbol {
            text: close.to_owned(),
            trailing: true,
            leading: false,
            overflow_left: 0.0,
            overflow_right: 1.0,
        });
    }
}

pub fn build() -> State<'static> {
    info!("build()");

    let storage = Storage::new();
    let mut document = Document::new(storage);
    symbols(&mut document);
    info!("reading font");
    let font_face = document.add_font(
        &include_bytes!("../../data/Cormorant-Regular.ttf")[..]
    );
    let math_font = document.add_font(
        &include_bytes!("../../data/latinmodern-math.otf")[..]
    );

    let mut document = ContentBuilder::with_document(document)
        .chapter().word("Test").finish()
        .paragraph()
            .text("The distilled spirit of Garamond")
            .finish()
        .paragraph()
            .text("The ffine fish")
            .finish()
        .paragraph()
            .text("Written in")
            .object(Object::Svg(SvgObject::new(Scale::FitTextHeight, include_bytes!("../../data/rust_logo.svg")[..].into())))
            .word("using")
            .object(Object::Svg(SvgObject::new(Scale::FitTextHeight, include_bytes!("../../data/pathfinder_logo.svg")[..].into())))
            .finish()
        .paragraph()
            .text("Using ReX to render")
            .object(Object::TeX(TeX::text(r#"T_e X"#, math_font)))
            .finish()
        .paragraph()
            .text("A inline equation")
            .object(Object::TeX(TeX::text(r#"\phi = \frac{1 + \sqrt{5}}{2}"#, math_font)))
            .finish()
        .paragraph()
            .text("And more text to collide with the previous equation.")
            .finish()
        .object(Object::TeX(TeX::display(r#"\frac{1}{\left(\sqrt{\phi\sqrt5} - \phi\right) e^{\frac{2}{5}\pi}} = 1 + \frac{e^{-2\pi}}{1 + \frac{e^{-4\pi}}{1 + \frac{e^{-6\pi}}{1 + \frac{e^{-8\pi}}{1 + \unicodecdots}}}}"#, math_font)))
        .object(Object::Svg(SvgObject::new(Scale::FitWidth, include_bytes!("../../data/Ghostscript_Tiger.svg")[..].into())))
        .finish();


    info!("done reading font");

    let hyphen = document.add_symbol(Symbol {
        text: "‐".into(),
        leading: false,
        trailing: true,
        overflow_left: 0.0,
        overflow_right: 1.0
    });

    let dictionary = document.load_dict(DICT_EN_GB);

    let default = TypeDesign {
        display:   Display::Inline,
        font:           Font {
            font_face,
            size:  Length::mm(4.0)
        },
        word_space: FlexMeasure {
            shrink:  Length::mm(1.0),
            length:  Length::mm(2.0),
            stretch: Length::mm(3.0)
        },
        line_height: Length::mm(5.0),
        indent:      Length::zero(),
        hyphen: Some(hyphen),
        dictionary
    };

    
    let mut design = Design::new("default design".into(), default);
    design.set_type(
        document.find_type("chapter").unwrap(),
        TypeDesign {
            display:        Display::Block(
                VerticalPadding {
                    above: Length::zero(),
                    below: Length::mm(4.0)
                }
            ),
            font:           Font {
                font_face,
                size:  Length::mm(8.0)
            },
            word_space: FlexMeasure {
                shrink:  Length::mm(2.0),
                length:   Length::mm(4.0),
                stretch: Length::mm(6.0)
            },
            line_height: Length::mm(10.0),
            indent:      Length::zero(),
            hyphen: None,
            dictionary
        }
    );
    design.set_type(
        document.find_type("paragraph").unwrap(),
        TypeDesign {
            display:        Display::Paragraph(
                Length::mm(10.0),
                VerticalPadding {
                    above: Length::zero(),
                    below: Length::mm(4.0)
                }
            ),
            font:           Font {
                font_face,
                size:  Length::mm(4.0)
            },
            word_space: FlexMeasure {
                shrink:  Length::mm(1.0),
                length:  Length::mm(2.0),
                stretch: Length::mm(3.0)
            },
            line_height: Length::mm(5.0),
            indent:      Length::zero(),
            hyphen: Some(hyphen),
            dictionary
        }
    );

    let target = default_target();
    State {
        root: document.root(),
        storage: Cow::Owned(document.into_storage()),
        design: Cow::Owned(design),
        target: Cow::Owned(target),
    }
}

pub fn default_target() -> Target {
    Target {
        description: "test target".into(),
        content_box: Rect {
            left: Length::mm(20.),
            width: Length::mm(130.),
            top: Length::mm(10.),
            height: Length::mm(220.)
        },
        media_box: Rect {
            left: Length::mm(-3.),
            width: Length::mm(176.),
            top: Length::mm(-3.),
            height: Length::mm(246.)
        },
        trim_box: Rect {
            left: Length::mm(0.),
            width: Length::mm(170.),
            top: Length::mm(0.),
            height: Length::mm(240.)
        },
        page_color: Color
    }
}
