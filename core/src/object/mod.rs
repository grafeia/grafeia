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

impl Object {
    pub fn size(&self, ctx: &DrawCtx) -> (FlexMeasure, Length) {
        match *self {
            Object::Svg(ref svg) => svg.size(ctx),
            Object::TeX(ref tex) => tex.size(ctx),
        }
    }
    pub fn draw(&self, typ: &TypeDesign, origin: Vector2F, size: Vector2F, scene: &mut Scene) {
        match *self {
            Object::Svg(ref svg) => svg.draw(typ, origin, size, scene),
            Object::TeX(ref tex) => tex.draw(typ, origin, size, scene),
        }
    }
}
