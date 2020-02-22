use serde::{Serialize, Deserialize, Serializer, ser::SerializeTuple};
use pathfinder_svg::BuiltSVG;
use pathfinder_geometry::{
    vector::Vector2F,
    rect::RectF,
    transform2d::Transform2F
};
use pathfinder_renderer::scene::{Scene, DrawPath};
use pathfinder_content::{
    fill::FillRule as PaFillRule,
    effects::BlendMode
};
use crate::*;
use crate::draw::DrawCtx;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Serialize, Deserialize, Clone)]
pub enum Object {
    Svg(SvgObject),
    TeX(TeX)
}
impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Object::Svg(_) => write!(f, "SVG"),
            Object::TeX(ref tex) => write!(f, "{}", tex.tex)
        }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub enum Scale {
    FitWidth,
    FitLineHight,
    FitTextHeight,
    Width(Length),
    Height(Length)
}

impl Object {
    pub fn size(&self, ctx: &DrawCtx) -> FlexMeasure {
        match *self {
            Object::Svg(ref svg) => svg.size(ctx),
            Object::TeX(ref tex) => tex.size(ctx),
        }
    }
    pub fn draw(&self, ctx: &DrawCtx, origin: Vector2F, size: Vector2F, scene: &mut Scene) {
        match *self {
            Object::Svg(ref svg) => svg.draw(ctx, origin, size, scene),
            Object::TeX(ref tex) => tex.draw(ctx, origin, size, scene),
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(from="(Scale, Vec<u8>)")]
pub struct SvgObject {
    data: Vec<u8>,
    scene: Scene,
    scale: Scale
}
impl Hash for SvgObject {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state)
    }
}
impl PartialEq for SvgObject {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
    }
}
impl Eq for SvgObject {}

impl SvgObject {
    pub fn new(scale: Scale, data: Vec<u8>) -> Self {
        use usvg::{Tree, Options};
        let tree = Tree::from_data(&data, &Options::default()).unwrap();
        let scene = BuiltSVG::from_tree(&tree).scene;

        SvgObject {
            scale,
            data,
            scene
        }
    }
}
impl From<(Scale, Vec<u8>)> for SvgObject {
    fn from((scale, data): (Scale, Vec<u8>)) -> Self {
        Self::new(scale, data)
    }
}
impl Serialize for SvgObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut t = serializer.serialize_tuple(2)?;
        t.serialize_element(&self.scale)?;
        t.serialize_element(&self.data)?;
        t.end()
    }
}
impl SvgObject {
    fn size(&self, ctx: &DrawCtx) -> FlexMeasure {
        let svg_size = self.scene.view_box().size();
        let (w, h) = match self.scale {
            Scale::FitWidth => (Some(ctx.target.content_box.width), None),
            Scale::FitLineHight => (None, Some(ctx.type_design.line_height)),
            Scale::FitTextHeight => (None, Some(ctx.type_design.font.size)),
            Scale::Width(w) => (Some(w), None),
            Scale::Height(h) => (None, Some(h))
        };
        let (w, h) = match (w, h) {
            (Some(w), None) => (w, w * (svg_size.y() / svg_size.x())),
            (None, Some(h)) => (h * (svg_size.x() / svg_size.y()), h),
            _ => unreachable!()
        };
        FlexMeasure::fixed_box(w, h)
    }

    fn draw(&self, _ctx: &DrawCtx, origin: Vector2F, size: Vector2F, scene: &mut Scene) {
        // coorinates are at the lower left, but objects expect the origin at the top left
        let view_box = self.scene.view_box();
        let svg_size = view_box.size();
        let scale = size.x() / svg_size.x();
        let tr = Transform2F::from_translation(origin)
            * Transform2F::from_scale(Vector2F::splat(scale))
            * Transform2F::from_translation(-view_box.lower_left());
        
        for (paint, outline, _) in self.scene.paths() {
            let outline = outline.clone();
            let new_path = DrawPath::new(
                outline.transform(tr),
                scene.push_paint(paint),
                None,
                PaFillRule::Winding,
                BlendMode::default(),
                String::new());
            scene.push_path(new_path);
        }
    }
}

pub use rex::layout::Style as TeXStyle;
use rex::{Renderer, RenderSettings, Cursor, parser::color::RGBA, fp::F24P8, layout::Layout, constants::UNITS_PER_EM};
use font::{OpenTypeFont, Font};
use vector::{Surface, PathBuilder, PathStyle, Outline, FillRule};

#[derive(Serialize, Deserialize, Clone)]
pub struct TeX {
    tex: String,
    style: TeXStyle,
}
lazy_static! {
    static ref XITS: OpenTypeFont<pathfinder_content::outline::Outline> = {
        OpenTypeFont::parse(&include_bytes!("../../data/rex-xits.otf")[..])
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
    fn build(&self, ctx: &DrawCtx) -> (Layout, RenderSettings) {
        let mut parse = rex::parser::parse(&self.tex).expect("invalid tex");
        let settings = RenderSettings::default()
            .font_size(ctx.type_design.font.size.value as f64)
            .horz_padding(0.into())
            .style(self.style);

        let layout = rex::layout::engine::layout(&mut parse, settings.layout_settings());
        (layout, settings)
    }
    fn size(&self, ctx: &DrawCtx) -> FlexMeasure {
        let (layout, settings) = self.build(ctx);
        let cvt = |size| Length::mm((settings.font_size * f64::from(size) / f64::from(UNITS_PER_EM)) as f32);

        // Left and right padding
        let width = layout.width + 2 * settings.horz_padding;
        // Top and bot padding
        let height = match self.style {
            TeXStyle::Display | TeXStyle::DisplayCramped => cvt(layout.height - layout.depth + 2 * settings.vert_padding),
            _ => ctx.type_design.line_height
        };

        FlexMeasure::fixed_box(cvt(width), height)
    }
    fn draw(&self, ctx: &DrawCtx, origin: Vector2F, size: Vector2F, scene: &mut Scene) {
        let (layout, settings) = self.build(ctx);
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
            style: scene.build_style(PathStyle {
                fill: Some((0, 0, 0, 200)),
                stroke: None, fill_rule:
                FillRule::NonZero
            }),
            bbox_style: scene.build_style(PathStyle {
                fill: Some((200, 0, 0, 200)),
                stroke: None, fill_rule:
                FillRule::NonZero
            }),
            transform,
            font: &*XITS
        };
        renderer.render_layout_to(scene, &layout);
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
        
        out.draw_path(glyph.path.transform(tr), &self.style);
    }

    fn bbox(&self, out: &mut Self::Out, pos: Cursor, width: F24P8, height: F24P8, _color: &str) {
        let rect = rect2rect(pos, width, height);

        let mut pb: PathBuilder<S::Outline> = PathBuilder::new();
        pb.rect(rect);
        
        let tr = self.transform;
        out.draw_path(pb.into_outline().transform(tr), &self.bbox_style);
    }

    fn rule(&self, out: &mut Self::Out, pos: Cursor, width: F24P8, height: F24P8) {
        let rect = rect2rect(pos, width, height);
        let mut pb: PathBuilder<S::Outline> = PathBuilder::new();
        pb.rect(rect);
        
        let tr = self.transform;
        out.draw_path(pb.into_outline().transform(tr), &self.style);
    }

    fn color<F>(&self, out: &mut Self::Out, color: RGBA, mut contents: F)
        where F: FnMut(&Self, &mut Self::Out)
    {
        let RGBA(r, g, b, a) = color;
        let style = out.build_style(PathStyle { fill: Some((r, g, b, a)), stroke: None, fill_rule: FillRule::NonZero });

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
