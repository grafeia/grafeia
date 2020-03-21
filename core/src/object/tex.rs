use serde::{Serialize, Deserialize};
use pathfinder_geometry::{
    vector::Vector2F,
    rect::RectF,
    transform2d::Transform2F
};
use pathfinder_renderer::scene::{Scene};
use crate::*;
use crate::object::DrawCtx;
use std::fmt;


pub use rex::layout::{
    Style as TeXStyle,
};
use rex::{
    Renderer, Cursor, parser::color::RGBA,
    layout::{Layout, LayoutSettings},
};
use rex::render::SceneWrapper;
use rex::font::FontContext;
use rex::dimensions::{self, Px};
use font::{OpenTypeFont, Font};
use vector::{Surface, PathBuilder, PathStyle, Outline, FillRule, Paint};

#[derive(Serialize, Deserialize, Clone)]
pub struct TeX {
    tex: String,
    style: TeXStyle,
    font: FontId
}
impl fmt::Debug for TeX {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.tex)
    }
}
lazy_static! {
    static ref XITS: OpenTypeFont<pathfinder_content::outline::Outline> = {
        OpenTypeFont::parse(data!("rex-xits.otf"))
    };
}
fn cvt(v: dimensions::Length<Px>) -> Length {
    Length::mm((v / Px) as f32)
}
impl TeX {
    pub fn display(tex: impl Into<String>, font: FontId) -> Self {
        TeX::new(tex, TeXStyle::Display, font)
    }
    pub fn text(tex: impl Into<String>, font: FontId) -> Self {
        TeX::new(tex, TeXStyle::Text, font)
    }
    pub fn new(tex: impl Into<String>, style: TeXStyle, font: FontId) -> Self {
        TeX {
            tex: tex.into(),
            style,
            font
        }
    }
    fn build<T>(&self, ctx: ObjectCtx, f: impl FnOnce(Layout, Renderer, &TypeDesign) -> T) -> T {
        let type_design = ctx.design.get_type_or_default(ctx.typ);
        let mut parse = rex::parser::parse(&self.tex).expect("invalid tex");
        let font = ctx.storage.get_font_face(self.font);
        let font = font.downcast().expect("not a OpenType font");
        let font_ctx = FontContext::new(font);
        let renderer = Renderer::new();
        let layout_settings = LayoutSettings::new(&font_ctx, type_design.font.size.value as f64, self.style);
        let layout = renderer.layout(&self.tex, layout_settings).unwrap();
        f(layout, renderer, type_design)
    }
    pub fn size(&self, ctx: ObjectCtx) -> (FlexMeasure, Length) {
        self.build(ctx, |layout, renderer, type_design| {
            let (x0, y0, x1, y1) = renderer.size(&layout);
            // Left and right padding
            let width = layout.width;
            // Top and bot padding
            let height = match self.style {
                TeXStyle::Display | TeXStyle::DisplayCramped => cvt(layout.height - layout.depth),
                _ => type_design.line_height
            };

            (FlexMeasure::fixed(cvt(width)), height)
        })
    }
    pub fn draw(&self, ctx: ObjectCtx, origin: Vector2F, size: Vector2F, surface: &mut Scene) {
        self.build(ctx, |layout, renderer, type_design| {
            let layout_width = layout.width / Px;
            let y_off = match self.style {
                TeXStyle::Display | TeXStyle::DisplayCramped => layout.depth / Px,
                _ => 0.0
            };

            dbg!(origin, size, layout.height, layout.depth);
            let transform = Transform2F::from_translation(origin + Vector2F::new(0.0, y_off as f32));
            
            let mut backend = SceneWrapper::with_transform(surface, transform);
            renderer.render(&layout, &mut backend);
        })
    }
}