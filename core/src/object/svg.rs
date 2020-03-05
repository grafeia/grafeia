use serde::{Serialize, Deserialize, Serializer, ser::SerializeTuple};
use pathfinder_svg::BuiltSVG;
use pathfinder_geometry::{
    vector::Vector2F,
    transform2d::Transform2F
};
use pathfinder_renderer::scene::{Scene, DrawPath};
use crate::*;
use crate::draw::DrawCtx;
use std::hash::{Hash, Hasher};
use std::fmt;
use vector::Outline;

#[derive(Deserialize, Clone)]
#[serde(from="(Scale, Vec<u8>)")]
pub struct SvgObject {
    data: Vec<u8>,
    scene: Scene,
    scale: Scale
}
impl fmt::Debug for SvgObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SVG")
    }
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
    pub fn size(&self, ctx: &DrawCtx) -> (FlexMeasure, Length) {
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
        (FlexMeasure::fixed(w), h)
    }

    pub fn draw(&self, _typ: &TypeDesign, origin: Vector2F, size: Vector2F, scene: &mut Scene) {
        // coorinates are at the lower left, but objects expect the origin at the top left
        let view_box = self.scene.view_box();
        let svg_size = view_box.size();
        let scale = size.x() / svg_size.x();
        let tr = Transform2F::from_translation(origin)
            * Transform2F::from_scale(Vector2F::splat(scale))
            * Transform2F::from_translation(-view_box.lower_left());
        
        for (paint, outline, _) in self.scene.paths() {
            let outline = outline.clone();
            let new_path = DrawPath::new(outline.transform(tr), scene.push_paint(paint));
            scene.push_path(new_path);
        }
    }
}