use serde::{Serialize, Deserialize};
use pathfinder_geometry::{
    vector::Vector2F,
};
use pathfinder_renderer::scene::{Scene};
use crate::*;
use crate::draw::DrawCtx;

pub mod svg;
pub mod tex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Object {
    Svg(svg::SvgObject),
    TeX(tex::TeX)
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

pub struct ObjectCtx<'a> {
    pub storage: &'a Storage,
    pub target: &'a Target,
    pub design: &'a Design,
    pub typ: TypeId,
}

impl Object {
    pub fn size(&self, ctx: ObjectCtx) -> (FlexMeasure, Length) {
        match *self {
            Object::Svg(ref svg) => svg.size(ctx),
            Object::TeX(ref tex) => tex.size(ctx),
        }
    }
    pub fn draw(&self, ctx: ObjectCtx, origin: Vector2F, size: Vector2F, scene: &mut Scene) {
        match *self {
            Object::Svg(ref svg) => svg.draw(ctx, origin, size, scene),
            Object::TeX(ref tex) => tex.draw(ctx, origin, size, scene),
        }
    }
}
