use crate::content::{Storage, Target, Item, Design, TypeDesign, Font, Word, Sequence};
use crate::layout::{Writer, Glue, Style, ColumnLayout, FlexMeasure};
use crate::units::Length;
use crate::Display;

use font;
use vector::{Vector, PathStyle, Surface, PathBuilder};
use pathfinder_geometry::{
    vector::Vector2F,
    rect::RectF,
    transform2d::Transform2F
};
use pathfinder_content::{
    outline::{Contour, Outline},
    color::ColorU
};
use pathfinder_renderer::{
    scene::{Scene, PathObject},
    paint::{Paint, PaintId}
};
#[derive(Clone)]
struct Context<'a> {
    storage:    &'a Storage,
    target:     &'a Target,
    design:     &'a Design,
    type_design:  &'a TypeDesign,
    root:       &'a Item
}

struct Layout {
    advance: Vector2F,
    glyphs: Vec<(font::GlyphId, Transform2F)>
}

pub struct Cache {

}
impl Cache {
    pub fn new() -> Cache {
        Cache {}
    }

    pub fn render(&mut self, storage: &Storage, target: &Target, item: &Item, design: &Design) -> Vec<Scene> {
        let mut writer = Writer::new();
        let type_design = design.default();
        let context = Context {
            storage,
            target,
            design,
            type_design,
            root: item
        };
        self.render_item(&mut writer, &context, item, 0);

        let mut scenes = Vec::new();
        let stream = writer.finish();
        let layout = ColumnLayout::new(&stream, target.content_box.width, target.content_box.height);
        for column in layout.columns() {
            let mut scene = Scene::new();
            scene.set_bounds(target.media_box.into());
            scene.set_view_box(target.trim_box.into());

            let style = scene.build_style(PathStyle {
                fill: Some((255,255,255,255)),
                stroke: Some(((0,0,0,255), 0.25))
            });
            let mut pb = PathBuilder::new();
            pb.rect(target.trim_box.into());
            
            scene.draw_path(pb.into_outline(), &style);

            let paint = scene.push_paint(&Paint { color: ColorU { r: 0, g: 0, b: 0, a: 255 } });
            use crate::layout::Item as LayoutItem;

            let mut line_indices = Vec::new();
            let content_box: RectF = target.content_box.into();
            for (y, line) in column {
                let mut line_items = Vec::new();
                for (x, item, &(font, idx)) in line {
                    match item {
                        LayoutItem::Word(key) => {
                            let word = storage.get_word(key);
                            let mut outline = self.render_word(word, storage, font);
                            let word_offset = Vector2F::new(x.value as f32, y.value as f32);
                            let tr = Transform2F::from_translation(content_box.origin() + word_offset);
                            outline.transform(&tr);
                            scene.push_path(PathObject::new(outline, paint, format!("{}", idx)));
                        }
                        _ => {}
                    }
                    line_items.push((x, idx));
                }
                line_indices.push((y, line_items));
            }


            scenes.push(scene);
        }

        scenes
    }

    fn render_item<'a>(&mut self, writer: &mut Writer<(&'a Font, usize)>, ctx: &Context<'a>, item: &Item, mut idx: usize) {
        assert_eq!(ctx.root.find(idx).unwrap() as *const _, item as *const _);

        match *item {
            Item::Word(key) => {
                let word = ctx.storage.get_word(key);
                let font = &ctx.type_design.font;
                let space = Glue::space(ctx.type_design.word_space);
                let width = self.word_layout(word, &ctx.storage, font).advance.x();
                let measure = FlexMeasure::fixed_box(Length::mm(width), ctx.type_design.line_height);

                writer.word(space, space, key, measure, (font, idx));
            }
            Item::Symbol(_) => {
            }
            Item::Sequence(ref seq) => {
                let type_design = ctx.design.get_type(seq.typ()).unwrap_or(ctx.design.default());
                let inner_context = Context {
                    type_design,
                    .. *ctx
                };
                match type_design.display {
                    Display::Block => writer.promote(Glue::newline()),
                    Display::Paragraph(indent) => writer.space(Glue::newline(), Glue::hfill(), FlexMeasure::fixed(indent), false),
                    _ => {}
                }
                idx += 1;
                for item in seq.items() {
                    self.render_item(writer, &inner_context, item, idx);
                    idx += item.num_nodes();
                }
                match type_design.display {
                    Display::Block => writer.promote(Glue::hfill()),
                    Display::Paragraph(_) => writer.promote(Glue::hfill()),
                    _ => {}
                }
            }
        }
    }
    fn word_layout(&mut self, word: &Word, storage: &Storage, font: &Font) -> Layout {
        use font::{GlyphId};

        let face = storage.get_font_face(font.font_face);
        let mut last_gid = None;
        let mut gids: Vec<GlyphId> = word.text.chars().map(|c| face.gid_for_unicode_codepoint(c as u32).unwrap_or(face.get_notdef_gid())).collect();
        if let Some(gsub) = face.get_gsub() {
            let mut substituted_gids = Vec::new();
            let mut pos = 0;
        'a: while let Some(&first) = gids.get(pos) {
                pos += 1;
                if let Some(subs) = gsub.substitutions(first) {
                for (sub, glyph) in subs {
                        if let Some(len) = sub.matches(&gids[pos ..]) {
                            substituted_gids.push(glyph);
                            pos += len;
                            continue 'a;
                        }
                    }
                }
    
                substituted_gids.push(first);
            }
    
            gids = substituted_gids;
        }

        let transform = Transform2F::from_scale(Vector2F::splat(font.size.value))
         * Transform2F::from_scale(Vector2F::new(1.0, -1.0))
         * face.font_matrix();
        
        let mut offset = Vector2F::default();
        let mut glyphs = Vec::with_capacity(gids.len());
        for &gid in gids.iter() {
            if let Some(glyph) = face.glyph(gid) {
                if let Some(left) = last_gid.replace(gid) {
                    offset = offset + Vector2F::new(face.kerning(left, gid), 0.0);
                }
                glyphs.push((gid, transform * Transform2F::from_translation(offset)));

                offset = offset + glyph.metrics.advance;
            }
        }

        Layout {
            advance: transform * offset,
            glyphs
        }
    }

    fn render_word(&mut self, word: &Word, storage: &Storage, font: &Font) -> Outline {
        let layout = self.word_layout(word, storage, font);
        let font = storage.get_font_face(font.font_face);

        let mut word_outline = Outline::new();
        for &(gid, tr) in layout.glyphs.iter() {
            if let Some(glyph) = font.glyph(gid) {
                for contour in glyph.path.contours() {
                    let mut contour = contour.clone();
                    contour.transform(&tr);
                    word_outline.push_contour(contour);
                }
            }
        }
        word_outline
    }
}