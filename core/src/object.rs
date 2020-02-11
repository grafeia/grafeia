use serde::{Serialize, Deserialize, Serializer, ser::SerializeTuple};
use pathfinder_svg::BuiltSVG;
use pathfinder_geometry::{
    vector::Vector2F,
    rect::RectF,
    transform2d::Transform2F
};
use pathfinder_renderer::scene::{Scene, PathObject};
use crate::*;
use crate::draw::DrawCtx;
use std::fmt;

#[derive(Serialize, Deserialize)]
pub enum Object {
    Svg(SvgObject),
    TeX
}
impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Object::Svg(_) => write!(f, "SVG"),
            Object::TeX => write!(f, "TeX")
        }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum Size {
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
            _ => FlexMeasure::zero()
        }
    }
    pub fn draw(&self, ctx: &DrawCtx, origin: Vector2F, width: Length, height: Length, scene: &mut Scene) {
        match *self {
            Object::Svg(ref svg) => svg.draw(ctx, origin, width, height, scene),
            Object::TeX => {}
        }
    }
}

#[derive(Deserialize)]
#[serde(from="(Size, Vec<u8>)")]
pub struct SvgObject {
    data: Vec<u8>,
    scene: Scene,
    size: Size
}
impl SvgObject {
    pub fn new(size: Size, data: Vec<u8>) -> Self {
        use usvg::{Tree, Options};
        let tree = Tree::from_data(&data, &Options::default()).unwrap();
        let scene = BuiltSVG::from_tree(tree).scene;

        SvgObject {
            size,
            data,
            scene
        }
    }
}
impl From<(Size, Vec<u8>)> for SvgObject {
    fn from((size, data): (Size, Vec<u8>)) -> Self {
        Self::new(size, data)
    }
}
impl Serialize for SvgObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut t = serializer.serialize_tuple(2)?;
        t.serialize_element(&self.size)?;
        t.serialize_element(&self.data)?;
        t.end()
    }
}
impl SvgObject {
    fn size(&self, ctx: &DrawCtx) -> FlexMeasure {
        let svg_size = self.scene.view_box().size();
        let (w, h) = match self.size {
            Size::FitWidth => (Some(ctx.target.content_box.width), None),
            Size::FitLineHight => (None, Some(ctx.type_design.line_height)),
            Size::FitTextHeight => (None, Some(ctx.type_design.font.size)),
            Size::Width(w) => (Some(w), None),
            Size::Height(h) => (None, Some(h))
        };
        let (w, h) = match (w, h) {
            (Some(w), None) => (w, w * (svg_size.y() / svg_size.x())),
            (None, Some(h)) => (h * (svg_size.x() / svg_size.y()), h),
            _ => unreachable!()
        };
        FlexMeasure::fixed_box(w, h)
    }

    fn draw(&self, ctx: &DrawCtx, origin: Vector2F, width: Length, height: Length, scene: &mut Scene) {
        // coorinates are at the lower left, but objects expect the origin at the top left
        let view_box = self.scene.view_box();
        let size = view_box.size();
        let scale = width.value / size.x();
        let tr = Transform2F::from_translation(origin)
            * Transform2F::from_scale(Vector2F::splat(scale))
            * Transform2F::from_translation(-view_box.lower_left());
        
        for (paint, outline, _) in self.scene.paths() {
            let mut outline = outline.clone();
            outline.transform(&tr);
            let new_path = PathObject::new(outline, scene.push_paint(paint), String::new());
            scene.push_path(new_path);
        }
    }
}
