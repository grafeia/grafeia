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
    outline::{Contour, Outline},
    color::ColorU
};
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
    glyphs: Vec<(font::GlyphId, Transform2F)>
}

pub struct Page {
    scene: Scene,
    tags: Vec<(f32, Vec<(f32, usize)>)>,
    positions: HashMap<usize, Vector2F>
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
                        return Some((Tag(tag), Vector2F::new(x, y)));
                    }
                }
            }
        }
        None
    }
    pub fn position(&self, Tag(idx): Tag) -> Option<Vector2F> {
        self.positions.get(&idx).cloned()
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
impl Cache {
    pub fn new() -> Cache {
        Cache {
            layout_cache: HashMap::new()
        }
    }

    pub fn render(&mut self, storage: &Storage, target: &Target, document: &Document, design: &Design) -> Vec<Page> {
        let mut writer = Writer::new();
        let type_design = design.default();
        let context = Context {
            storage,
            target,
            type_design,
            document,
            design
        };
        self.render_sequence(&mut writer, &context, document.root(), 0);

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
            let mut positions = HashMap::new();
            let content_box: RectF = target.content_box.into();
            for (y, line) in column {
                let mut line_items = Vec::new();
                for (x, item, &(font, idx)) in line {
                    let p = content_box.origin() + Vector2F::new(x.value as f32, y.value as f32);
                    match item {
                        LayoutItem::Word(key) => {
                            let mut outline = self.render_word(key, storage, *font);
                            outline.transform(&Transform2F::from_translation(p));
                            scene.push_path(PathObject::new(outline, paint, format!("{}", idx)));
                        }
                        _ => {}
                    }
                    line_items.push((x.value + content_box.origin().x(), idx));
                    positions.insert(idx, p);
                }
                line_indices.push((y.value + content_box.origin().y(), line_items));
            }


            pages.push(Page { scene, tags: line_indices, positions });
        }

        pages
    }

    fn render_sequence<'a>(&mut self, writer: &mut Writer<(&'a Font, usize)>, ctx: &Context<'a>, seq: &Sequence, mut idx: usize) {
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
    fn render_item<'a>(&mut self, writer: &mut Writer<(&'a Font, usize)>, ctx: &Context<'a>, item: &Item, idx: usize) {
        assert_eq!(ctx.document.find(Tag(idx)).unwrap().1 as *const _, item as *const _);

        match *item {
            Item::Word(key) => {
                let font = &ctx.type_design.font;
                let space = Glue::space(ctx.type_design.word_space);
                let width = self.word_layout(key, &ctx.storage, *font).advance.x();
                let measure = FlexMeasure::fixed_box(Length::mm(width), ctx.type_design.line_height);

                writer.word(space, space, key, measure, (font, idx));
            }
            Item::Symbol(_) => {}
            Item::Sequence(ref seq) => self.render_sequence(writer, ctx, seq, idx+1)
        }
    }
    pub fn get_position_on_page(&self, storage: &Storage, design: &Design, document: &Document, page: &Page, tag: Tag, byte_pos: usize) -> Option<(Vector2F, TypeKey)> {
        match document.find(tag)? {
            (seq, &Item::Word(key)) => {
                let item_pos = page.position(tag)?;
                let type_design = design.get_type_or_default(seq.typ());
                let word = storage.get_word(key);
                let face = storage.get_font_face(type_design.font.font_face);
                let grapheme_indices = grapheme_indices(face, &word.text);
                let layout = self.layout_cache.get(&(type_design.font, key)).unwrap();

                for (&n, &(gid, offset)) in grapheme_indices.iter().zip(layout.glyphs.iter()) {
                    if n >= byte_pos {
                        return Some((item_pos + offset.vector, seq.typ()));
                    }
                }

                // point to the end
                return Some((item_pos + layout.advance, seq.typ()));
            }
            (parent, &Item::Sequence(ref seq)) => {
                for (tag, item) in document.items(tag .. tag + seq.num_nodes()) {
                    match *item {
                        Item::Word(_) => {
                            let item_pos = page.position(tag)?;
                            return Some((item_pos, seq.typ()));
                        }
                        _ => {}
                    }
                }
                None
            }
            _ => None
        }
    }
    pub fn get_rect_on_page(&self, storage: &Storage, design: &Design, document: &Document, page: &Page, tag: Tag) -> Option<(RectF, TypeKey)> {
        // first step is to locate the item on the page
        let item_pos = page.position(tag)?;

        // then get the layout of the word
        if let (seq, &Item::Word(key)) = document.find(tag)? {
            let type_design = design.get_type_or_default(seq.typ());
            let layout = self.layout_cache.get(&(type_design.font, key)).unwrap();

            // point to the end
            return Some((RectF::new(item_pos, layout.advance), seq.typ()));
        }
        None
    }
    pub fn find(&self, storage: &Storage, design: &Design, document: &Document, offset: f32, tag: Tag) -> Option<(Vector2F, usize, TypeKey)> {
        if let (seq, &Item::Word(key)) = document.find(tag)? {
            let type_design = design.get_type_or_default(seq.typ());
            let word = storage.get_word(key);
            let face = storage.get_font_face(type_design.font.font_face);
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

    fn render_word(&mut self, word: WordKey, storage: &Storage, font: Font) -> Outline {
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