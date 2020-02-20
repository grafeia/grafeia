use crate::app::App;
use grafeia_core::*;
use grafeia_core::builder::*;
use std::borrow::Cow;

pub fn build() -> App {
    info!("build()");

    let mut document = ContentBuilder::new()
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
            .object(Object::TeX(TeX::text(r#"T_e X"#)))
            .finish()
        .paragraph()
            .text("A inline equation")
            .object(Object::TeX(TeX::text(r#"\phi = \frac{1 + \sqrt{5}}{2}"#)))
            .finish()
        .paragraph()
            .text("And more text to collide with the previous equation.")
            .finish()
        .object(Object::TeX(TeX::display(r#"\frac{1}{\left(\sqrt{\phi\sqrt5} - \phi\right) e^{\frac{2}{5}\pi}} = 1 + \frac{e^{-2\pi}}{1 + \frac{e^{-4\pi}}{1 + \frac{e^{-6\pi}}{1 + \frac{e^{-8\pi}}{1 + \unicodecdots}}}}"#)))
        .object(Object::Svg(SvgObject::new(Scale::FitWidth, include_bytes!("../../data/Ghostscript_Tiger.svg")[..].into())))
        .finish();

    info!("reading font");
    let font_face = document.add_font(
        &include_bytes!("../../data/Cormorant-Regular.ttf")[..]
    );

    info!("done reading font");

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
        document.find_type("chapter").unwrap(),
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
        document.find_type("paragraph").unwrap(),
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

    let target = default_target();
    let state = State {
        root: document.root(),
        storage: Cow::Owned(document.into_storage()),
        design: Cow::Owned(design),
        target: Cow::Owned(target),
    };
    App::from_state(state, SiteId(1))
}

pub fn default_target() -> Target {
    Target {
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
    }
}
