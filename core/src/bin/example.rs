use grafeia_core::{
    content::*,
    units::*,
    builder::ContentBuilder,
    draw::Cache,
    layout::FlexMeasure,
    Color, Display
};
use font;
use pathfinder_export::{Export, FileFormat};

fn main() {
    let mut storage = Storage::new();
    let document = ContentBuilder::new(&mut storage)
        .chapter().word("Test").finish()
        .paragraph()
            .word("This")
            .word("is")
            .word("a")
            .word("test")
            .finish()
        .finish();
    
    let target = Target {
        description: "test target".into(),
        content_box: Rect {
            left: Length::mm(10.),
            width: Length::mm(150.),
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
    };

    let font_face = storage.insert_font_face(
        font::parse(&std::fs::read("/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf").unwrap())
    );

    let default = TypeDesign {
        display:   Display::Inline,
        font:           Font {
            font_face,
            size:  Length::mm(4.0)
        },
        word_space: FlexMeasure {
            height:  Length::zero(),
            shrink:  Length::mm(1.0),
            width:   Length::mm(2.0),
            stretch: Length::mm(3.0)
        },
        line_height: Length::mm(5.0)
    };

    
    let mut design = Design::new("default design".into(), default);
    design.set_type(
        storage.find_type("chapter").unwrap(),
        TypeDesign {
            display:        Display::Block,
            font:           Font {
                font_face,
                size:  Length::mm(8.0)
            },
            word_space: FlexMeasure {
                height:  Length::zero(),
                shrink:  Length::mm(2.0),
                width:   Length::mm(4.0),
                stretch: Length::mm(6.0)
            },
            line_height: Length::mm(10.0)
        }
    );
    design.set_type(
        storage.find_type("paragraph").unwrap(),
        TypeDesign {
            display:        Display::Paragraph(Length::mm(10.0)),
            font:           Font {
                font_face,
                size:  Length::mm(4.0)
            },
            word_space: FlexMeasure {
                height:  Length::zero(),
                shrink:  Length::mm(1.0),
                width:   Length::mm(2.0),
                stretch: Length::mm(3.0)
            },
            line_height: Length::mm(5.0)
        }
    );

    let mut cache = Cache::new();
    let pages = cache.render(&storage, &target, &document, &design);

    for (page_nr, page) in pages.iter().enumerate() {
        let mut file = std::fs::File::create(&format!("page_{}.svg", page_nr)).unwrap();
        page.export(&mut file, FileFormat::SVG).unwrap();
    }
}