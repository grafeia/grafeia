use serde::{Serialize, Deserialize};
use pathfinder_geometry::{
    vector::Vector2F,
    rect::RectF,
    transform2d::Transform2F
};
use pathfinder_renderer::scene::{Scene};
use crate::*;
use crate::draw::DrawCtx;
use std::fmt;


pub use rex::layout::Style as TeXStyle;
use rex::{Renderer, RenderSettings, Cursor, parser::color::RGBA, fp::F24P8, layout::Layout, constants::UNITS_PER_EM};
use font::{OpenTypeFont, Font};
use vector::{Surface, PathBuilder, PathStyle, Outline, FillRule, Paint};

#[derive(Serialize, Deserialize, Clone)]
pub struct TeX {
    tex: String,
    style: TeXStyle,
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
impl TeX {
    pub fn display(tex: impl Into<String>) -> Self {
        TeX::new(tex, TeXStyle::Display)
    }
    pub fn text(tex: impl Into<String>) -> Self {
        TeX::new(tex, TeXStyle::Text)
    }
    pub fn new(tex: impl Into<String>, style: TeXStyle) -> Self {
        TeX {
            tex: tex.into(),
            style
        }
    }
    fn build(&self, typ: &TypeDesign) -> (Layout, RenderSettings) {
        let mut parse = rex::parser::parse(&self.tex).expect("invalid tex");
        let settings = RenderSettings::default()
            .font_size(typ.font.size.value as f64)
            .horz_padding(0.into())
            .style(self.style);

        let layout = rex::layout::engine::layout(&mut parse, settings.layout_settings());
        (layout, settings)
    }
    pub fn size(&self, ctx: &DrawCtx) -> (FlexMeasure, Length) {
        let (layout, settings) = self.build(&ctx.type_design);
        let cvt = |size| Length::mm((settings.font_size * f64::from(size) / f64::from(UNITS_PER_EM)) as f32);

        // Left and right padding
        let width = layout.width + 2 * settings.horz_padding;
        // Top and bot padding
        let height = match self.style {
            TeXStyle::Display | TeXStyle::DisplayCramped => cvt(layout.height - layout.depth + 2 * settings.vert_padding),
            _ => ctx.type_design.line_height
        };

        (FlexMeasure::fixed(cvt(width)), height)
    }
    pub fn draw(&self, typ: &TypeDesign, origin: Vector2F, size: Vector2F, surface: &mut Scene) {
        let (layout, settings) = self.build(typ);
        let layout_width = layout.width + 2 * settings.horz_padding;
        let layout_height = layout.height - layout.depth + 2 * settings.vert_padding;
        let y_off = match self.style {
            TeXStyle::Display | TeXStyle::DisplayCramped => layout_height,
            _ => layout.height + settings.vert_padding
        };

        let scale = settings.font_size / f64::from(UNITS_PER_EM);
        let transform = Transform2F::from_translation(origin + Vector2F::new(size.x(), 0.0))
            * Transform2F::from_scale(Vector2F::splat(scale as f32))
            * Transform2F::from_translation(Vector2F::new(layout_width.bits as f32, y_off.bits as f32).scale(-1.0 / 256.));
        
        let renderer = TexRenderer {
            settings: &settings,
            style: surface.build_style(PathStyle {
                fill: Some(Paint::black()),
                stroke: None, fill_rule:
                FillRule::NonZero
            }),
            bbox_style: surface.build_style(PathStyle {
                fill: Some(Paint::Solid((200, 0, 0, 200))),
                stroke: None, fill_rule:
                FillRule::NonZero
            }),
            transform,
            font: &*XITS
        };
        renderer.render_layout_to(surface, &layout);
    }
}
struct TexRenderer<'a, S: Surface> {
    settings: &'a RenderSettings,
    style: S::Style,
    bbox_style: S::Style,
    transform: Transform2F,
    font: &'a OpenTypeFont<S::Outline>,
}
fn cursor2vec(Cursor { x, y }: Cursor) -> Vector2F {
    Vector2F::new(x.bits as f32, y.bits as f32).scale(1.0 / 256.)
}
fn rect2rect(cursor: Cursor, width: F24P8, height: F24P8) -> RectF {
    let origin = Vector2F::new(cursor.x.bits as f32, cursor.y.bits as f32);
    let size = Vector2F::new(width.bits as f32, height.bits as f32);
    RectF::new(origin, size).scale(1.0 / 256.)
}
impl<'a, S: Surface> Renderer for TexRenderer<'a, S> {
    type Out = S;
    fn symbol(&self, out: &mut Self::Out, pos: Cursor, symbol: u32, scale: f64) {
        let gid = self.font.gid_for_unicode_codepoint(symbol).unwrap();
        let glyph = self.font.glyph(gid).unwrap();
        let tr = self.transform
            * Transform2F::from_translation(cursor2vec(pos))
            * Transform2F::from_scale(Vector2F::new(1.0, -1.0).scale(scale as f32));
        
        out.draw_path(glyph.path.transform(tr), &self.style, None);
    }

    fn bbox(&self, out: &mut Self::Out, pos: Cursor, width: F24P8, height: F24P8, _color: &str) {
        let rect = rect2rect(pos, width, height);

        let mut pb: PathBuilder<S::Outline> = PathBuilder::new();
        pb.rect(rect);
        
        let tr = self.transform;
        out.draw_path(pb.into_outline().transform(tr), &self.bbox_style, None);
    }

    fn rule(&self, out: &mut Self::Out, pos: Cursor, width: F24P8, height: F24P8) {
        let rect = rect2rect(pos, width, height);
        let mut pb: PathBuilder<S::Outline> = PathBuilder::new();
        pb.rect(rect);
        
        let tr = self.transform;
        out.draw_path(pb.into_outline().transform(tr), &self.style, None);
    }

    fn color<F>(&self, out: &mut Self::Out, color: RGBA, mut contents: F)
        where F: FnMut(&Self, &mut Self::Out)
    {
        let RGBA(r, g, b, a) = color;
        let style = out.build_style(PathStyle { fill: Some(Paint::Solid((r, g, b, a))), stroke: None, fill_rule: FillRule::NonZero });

        contents(&TexRenderer {
            style,
            bbox_style: self.bbox_style.clone(),
            .. *self
        }, out);
    }
    fn settings(&self) -> &RenderSettings {
        self.settings
    }
}
