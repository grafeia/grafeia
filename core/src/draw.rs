use crate::*;
use crate::layout::{Writer, Glue, Style, ColumnLayout, FlexMeasure};
use crate::units::Length;
use crate::text::{grapheme_indices, build_gids};
use std::collections::hash_map::{HashMap, Entry};
use font;
use vector::{Vector, PathStyle, Surface, PathBuilder};
use pathfinder_geometry::{
    vector::Vector2F,
    rect::RectF,
    transform2d::Transform2F
};
use pathfinder_content::{
    outline::{Contour, Outline}
};
use pathfinder_color::ColorU;
use pathfinder_renderer::{
    scene::{Scene, PathObject},
    paint::{Paint, PaintId}
};

#[derive(Clone)]
struct Context<'a> {
    storage:     &'a Storage,
    target:      &'a Target,
    type_design: &'a TypeDesign,
    document:    &'a Document,
    design:      &'a Design,
}

#[derive(Debug)]
struct Layout {
    advance: Vector2F,
    glyphs: Vec<(font::GlyphId, Transform2F)>,
}

pub struct Page {
    scene: Scene,
    tags: Vec<(f32, Vec<(f32, Tag)>)>,
    positions: HashMap<Tag, RectF>
}
impl Page {
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn find(&self, p: Vector2F) -> Option<(Tag, Vector2F)> {
        // find the first line with y value greater than p.y
        for &(y, ref line) in self.tags.iter() {
            if y > p.y() {
                for &(x, tag) in line.iter().rev() {
                    if x <= p.x() {
                        return Some((tag, Vector2F::new(x, y)));
                    }
                }
            }
        }
        None
    }
    pub fn position(&self, tag: Tag) -> Option<RectF> {
        self.positions.get(&tag).cloned()
    }
}


pub struct Cache {
    layout_cache: HashMap<(Font, WordKey), Layout>
}
impl Default for Cache {
    fn default() -> Self {
        Cache::new()
    }
}
pub struct DrawCtx<'a> {
    pub storage: &'a Storage,
    pub target: &'a Target,
    pub design: &'a Design,
    pub type_design: &'a TypeDesign
}
impl Cache {
    pub fn new() -> Cache {
        Cache {
            layout_cache: HashMap::new()
        }
    }

    fn render_item(&mut self, writer: &mut Writer, ctx: &DrawCtx, tag: Tag, item: &Item) {
        match *item {
            Item::Word(key) => {
                let font = ctx.type_design.font;
                let space = Glue::space(ctx.type_design.word_space);
                let width = self.word_layout(key, &ctx.storage, font).advance.x();
                let measure = FlexMeasure::fixed_box(Length::mm(width), ctx.type_design.line_height);

                writer.word(space, space, key, measure, font, tag);
            }
            Item::Object(key) => {
                let obj = ctx.storage.get_object(key);
                let size = obj.size(ctx);
                writer.object(Glue::any(), Glue::any(), key, size, tag);
            }
            Item::Sequence(key) => self.render_sequence(writer, ctx, key),
            _ => {}
        }
    }
    fn render_sequence(&mut self, writer: &mut Writer, ctx: &DrawCtx, key: SequenceKey) {
        let seq = ctx.storage.get_sequence(key);
        match ctx.type_design.display {
            Display::Block => writer.promote(Glue::newline()),
            Display::Paragraph(indent) => writer.space(Glue::newline(), Glue::hfill(), FlexMeasure::fixed(indent), false),
            _ => {}
        }
        let ctx = DrawCtx {
            type_design: ctx.design.get_type_or_default(seq.typ()),
            .. *ctx
        };
        if seq.items.len() > 0 {
            for (i, item) in seq.items.iter().enumerate() {
                self.render_item(writer, &ctx, Tag::at(key, i), item);
            }
        } else {
            writer.empty(Glue::any(), Glue::any(), Tag::end(key));
        }
        match ctx.type_design.display {
            Display::Block | Display::Paragraph(_) => writer.promote(Glue::hfill()),
            _ => {}
        }
    }

    pub fn render(&mut self, target: &Target, document: &LocalDocument, design: &Design) -> Vec<Page> {
        let mut writer = Writer::new();
        let storage = &**document;
        let ctx = DrawCtx {
            storage,
            target,
            design,
            type_design: design.default()
        };
        self.render_sequence(&mut writer, &ctx, document.root());

        let mut pages = Vec::new();
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

            let paint = scene.push_paint(&Paint::Color(ColorU { r: 0, g: 0, b: 0, a: 200 }));
            use crate::layout::Item as LayoutItem;

            let mut line_indices = Vec::new();
            let mut positions = HashMap::new();
            let content_box: RectF = target.content_box.into();
            for (y, line) in column {
                let line_height = line.height();
                let mut line_items = Vec::new();
                for (x, size, item, tag) in line {
                    let size: Vector2F = size.into();
                    let p = content_box.origin() + Vector2F::new(x.value as f32, y.value as f32);
                    match item {
                        LayoutItem::Word(key, font) => {
                            let (mut outline, advance) = self.render_word(key, storage, font);
                            outline.transform(&Transform2F::from_translation(p));
                            scene.push_path(PathObject::new(outline, paint, String::new()));
                        }
                        LayoutItem::Object(key) => {
                            storage.get_object(key).draw(&ctx, p, size.into(), &mut scene);
                        }
                        LayoutItem::Empty => {
                            let mut pb = PathBuilder::new();
                            pb.move_to(p);
                            pb.line_to(p - Vector2F::new(0.0, line_height.value));
                            scene.draw_path(pb.into_outline(), &style);
                        }
                        _ => {}
                    };
                    line_items.push((x.value + content_box.origin().x(), tag));
                    positions.insert(tag, RectF::new(p - Vector2F::new(0.0, size.y()), size));
                }
                line_indices.push((y.value + content_box.origin().y(), line_items));
            }


            pages.push(Page { scene, tags: line_indices, positions });
        }

        pages
    }

    pub fn get_position_on_page(&self, design: &Design, document: &LocalDocument, page: &Page, tag: Tag, byte_pos: usize) -> Option<Vector2F> {
        match *document.get_item(tag)? {
            Item::Word(key) => {
                let rect = page.position(tag)?;
                if byte_pos == 0 {
                    return Some(rect.lower_left());
                }
                let seq = document.get_sequence(tag.seq);
                let type_design = design.get_type_or_default(seq.typ());
                let word = document.get_word(key);
                let face = document.get_font_face(type_design.font.font_face);
                let grapheme_indices = grapheme_indices(face, &word.text);
                let layout = self.layout_cache.get(&(type_design.font, key)).unwrap();

                let interpolate = |(idx_a, pos_a), (idx_b, pos_b), idx| -> Vector2F {
                    if idx == idx_b { return pos_b; }
                    let f = (idx - idx_a) as f32 / (idx_b - idx_a) as f32;
                    pos_a + (pos_b - pos_a).scale(f)
                };

                let mut last = (0, Vector2F::default());
                for (&n, &(gid, offset)) in grapheme_indices.iter().zip(layout.glyphs.iter()) {
                    if n >= byte_pos {
                        return Some(rect.lower_left() + interpolate(last, (n, offset.vector,), byte_pos));
                    }
                    last = (n, offset.vector);
                }

                Some(rect.lower_left() + interpolate(last, (word.text.len(), layout.advance), byte_pos))
            }
            Item::Sequence(key) => {
                let first = document.get_first(key);
                let rect = page.position(first)?;
                return Some(rect.lower_left());
            }
            _ => None
        }
    }
    pub fn find(&self, design: &Design, document: &LocalDocument, offset: f32, tag: Tag) -> Option<(Vector2F, usize, TypeKey)> {
        if let &Item::Word(key) = document.get_item(tag)? {
            let seq = document.get_sequence(tag.seq);
            let type_design = design.get_type_or_default(seq.typ());
            let word = document.get_word(key);
            let face = document.get_font_face(type_design.font.font_face);
            let grapheme_indices = grapheme_indices(face, &word.text);
            let layout = self.layout_cache.get(&(type_design.font, key)).unwrap();

            if offset >= layout.advance.x() {
                return Some((layout.advance, word.text.len(), seq.typ()));
            }
            for (&n, (gid, tr)) in grapheme_indices.iter().rev().zip(layout.glyphs.iter().rev()) {
                if tr.vector.x() < offset {
                    println!("{}|{}", &word.text[0..n], &word.text[n..]);
                    return Some((tr.vector, n, seq.typ()));
                }
            }
        }
        None
    }
    fn word_layout(&mut self, word: WordKey, storage: &Storage, font: Font) -> &Layout {
        self.layout_cache.entry((font, word))
            .or_insert_with(|| Cache::build_word_layout(word, storage, font))
    }

    fn build_word_layout(word: WordKey, storage: &Storage, font: Font) -> Layout {
        let word = storage.get_word(word);
        let face = storage.get_font_face(font.font_face);
        let mut last_gid = None;
        let gids = build_gids(face, &word.text);

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

    fn render_word(&mut self, word: WordKey, storage: &Storage, font: Font) -> (Outline, Vector2F) {
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
        (word_outline, layout.advance)
    }
}