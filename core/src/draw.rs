use crate::content::{Storage, Target, Item, Design, TypeDesign, Font, Word, Sequence, WordKey};
use crate::layout::{Writer, Glue, Style, ColumnLayout, FlexMeasure};
use crate::units::Length;
use crate::Display;
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
    outline::{Contour, Outline},
    color::ColorU
};
use pathfinder_renderer::{
    scene::{Scene, PathObject},
    paint::{Paint, PaintId}
};

#[derive(Clone)]
struct Context<'a, 'b> {
    storage:    &'b Storage,
    target:     &'b Target,
    type_design: &'a TypeDesign,
    root:       &'b Sequence
}

struct Layout {
    advance: Vector2F,
    glyphs: Vec<(font::GlyphId, Transform2F)>
}

pub struct Page {
    scene: Scene,
    tags: Vec<(f32, Vec<(f32, usize)>)>
}
impl Page {
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn find(&self, p: Vector2F) -> Option<(usize, (f32, f32))> {
        // find the first line with y value greater than p.y
        for &(y, ref line) in self.tags.iter() {
            if y > p.y() {
                for &(x, tag) in line.iter().rev() {
                    if x <= p.x() {
                        return Some((tag, (x, y)));
                    }
                }
            }
        }
        None
    }
}
pub struct Cache<'a> {
    design: &'a Design,
    layout_cache: HashMap<(&'a Font, WordKey), Layout>
}
impl<'a> Cache<'a> {
    pub fn new(design: &'a Design) -> Cache<'a> {
        Cache {
            design,
            layout_cache: HashMap::new()
        }
    }

    pub fn render(&mut self, storage: &Storage, target: &Target, root: &Sequence) -> Vec<Page> {
        let mut writer = Writer::new();
        let type_design = self.design.default();
        let context = Context {
            storage,
            target,
            type_design,
            root
        };
        self.render_sequence(&mut writer, &context, root, 0);

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

            let paint = scene.push_paint(&Paint { color: ColorU { r: 0, g: 0, b: 0, a: 255 } });
            use crate::layout::Item as LayoutItem;

            let mut line_indices = Vec::new();
            let content_box: RectF = target.content_box.into();
            for (y, line) in column {
                let mut line_items = Vec::new();
                for (x, item, &(font, idx)) in line {
                    match item {
                        LayoutItem::Word(key) => {
                            let mut outline = self.render_word(key, storage, font);
                            let word_offset = Vector2F::new(x.value as f32, y.value as f32);
                            let tr = Transform2F::from_translation(content_box.origin() + word_offset);
                            outline.transform(&tr);
                            scene.push_path(PathObject::new(outline, paint, format!("{}", idx)));
                        }
                        _ => {}
                    }
                    line_items.push((x.value + content_box.origin().x(), idx));
                }
                line_indices.push((y.value + content_box.origin().y(), line_items));
            }


            pages.push(Page { scene, tags: line_indices });
        }

        pages
    }

    fn render_sequence<'b>(&mut self, writer: &mut Writer<(&'a Font, usize)>, ctx: &Context<'a, 'b>, seq: &Sequence, mut idx: usize) {
        let type_design = self.design.get_type(seq.typ()).unwrap_or(self.design.default());
        let inner_context = Context {
            type_design,
            .. *ctx
        };
        match type_design.display {
            Display::Block => writer.promote(Glue::newline()),
            Display::Paragraph(indent) => writer.space(Glue::newline(), Glue::hfill(), FlexMeasure::fixed(indent), false),
            _ => {}
        }
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
    fn render_item<'b>(&mut self, writer: &mut Writer<(&'a Font, usize)>, ctx: &Context<'a, 'b>, item: &Item, idx: usize) {
        assert_eq!(ctx.root.find(idx).unwrap() as *const _, item as *const _);

        match *item {
            Item::Word(key) => {
                let font = &ctx.type_design.font;
                let space = Glue::space(ctx.type_design.word_space);
                let width = self.word_layout(key, &ctx.storage, font).advance.x();
                let measure = FlexMeasure::fixed_box(Length::mm(width), ctx.type_design.line_height);

                writer.word(space, space, key, measure, (font, idx));
            }
            Item::Symbol(_) => {}
            Item::Sequence(ref seq) => self.render_sequence(writer, ctx, seq, idx+1)
        }
    }
    pub fn find(&self, storage: &Storage, design: &Design, seq: &Sequence, offset: f32, mut idx: usize) {
        for item in seq.items() {
            match *item {
                Item::Word(key) => {
                    if idx == 0 {
                        let type_design = design.get_type(seq.typ()).unwrap_or(design.default());
                        let word = storage.get_word(key);
                        let face = storage.get_font_face(type_design.font.font_face);
                        let grapheme_indices = grapheme_indices(face, &word.text);
                        let layout = self.layout_cache.get(&(&type_design.font, key)).unwrap();
                        for (&n, (gid, tr)) in grapheme_indices.iter().zip(layout.glyphs.iter()) {
                            if tr.vector.x() < offset {
                                println!("{}|{}", &word.text[0..n], &word.text[n..]);
                                break;
                            }
                        }
                        return;
                    }
                    idx -= 1;
                }
                Item::Symbol(_) => {
                    if idx == 0 {
                        return;
                    }
                    idx -= 1;
                }
                Item::Sequence(ref seq) => {
                    if idx == 0 || idx == seq.num_nodes() + 1 {
                        // found. it is the sequence itself
                        return;
                    }
                    idx -= 1;
                    if idx < seq.num_nodes() {
                        // within the sequence
                        return self.find(storage, design, seq, offset, idx);
                    }
                    idx -= seq.num_nodes() + 1;
                }
            }
        }
    }
    fn word_layout(&mut self, word: WordKey, storage: &Storage, font: &'a Font) -> &Layout {
        if let Some(layout) = self.layout_cache.get(&(font, word)) {
            return layout;
        }
        let layout = self.build_word_layout(word, storage, font);
        match self.layout_cache.entry((font, word)) {
            Entry::Vacant(e) => e.insert(layout),
            _ => unreachable!()
        }
    }

    fn build_word_layout(&self, word: WordKey, storage: &Storage, font: &Font) -> Layout {
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

    fn render_word(&mut self, word: WordKey, storage: &Storage, font: &'a Font) -> Outline {
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