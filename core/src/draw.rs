use crate::*;
use crate::layout::{Writer, Glue, ColumnLayout, FlexMeasure, Column, Columns, ItemMeasure};
use crate::units::Length;
use crate::text::{grapheme_indices, build_gids};
use std::collections::hash_map::{HashMap};
use font;
use vector::{PathStyle, Surface, PathBuilder, FillRule};
use pathfinder_geometry::{
    vector::Vector2F,
    rect::RectF,
    transform2d::Transform2F
};
use pathfinder_content::outline::Outline;
use pathfinder_renderer::scene::Scene;

#[inline]
fn select<T>(cond: bool, a: T, b: T) -> T {
    if cond { a } else { b }
}

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
impl Layout {
    fn render(&self, font: &FontFace, root_tr: Transform2F) -> Outline {
        let mut outline = Outline::new();
        for &(gid, tr) in self.glyphs.iter() {
            if let Some(glyph) = font.glyph(gid) {
                for contour in glyph.path.contours() {
                    let mut contour = contour.clone();
                    contour.transform(&(root_tr * tr));
                    outline.push_contour(contour);
                }
            }
        }
        outline
    }
}

#[derive(Hash, PartialEq, Eq)]
pub enum Marker {
    Start(SequenceId),
    End(SequenceId),
}
pub enum RenderedWord {
    Full(RectF),
    // part before hyphenation, part after hyphenation, index at which the word was broken
    Before(RectF, u16),
    After(RectF, u16),
    Both(RectF, RectF, u16)
}
pub struct Pages {
    pub columns: Columns,
}
pub struct Page {
    scene: Scene,
    items: Vec<(f32, Vec<(f32, Tag)>)>,
    pub positions: HashMap<Tag, RectF>,
    pub word_positions: HashMap<Tag, RenderedWord>,
}
impl Page {
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn find(&self, p: Vector2F) -> Option<(Tag, Vector2F)> {
        // find the first line with y value greater than p.y
        for &(y, ref line) in self.items.iter() {
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
}

#[derive(Debug, Copy, Clone)]
pub enum RenderItem {
    Word(WordId, WordPart, Font),
    Symbol(SymbolId, Font),
    Object(ObjectId),
    Empty,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum WordPart {
    Full,
    Before(u16),
    After(u16)
}

pub struct Cache {
    word_layout_cache: HashMap<(Font, WordId, WordPart), Layout>,
    symbol_layout_cache: HashMap<(Font, SymbolId), Layout>
}
impl Default for Cache {
    fn default() -> Self {
        Cache::new()
    }
}
pub struct DrawCtx<'a> {
    pub storage: &'a Storage,
    pub design: &'a Design,
    pub target: &'a Target,
    pub type_design: &'a TypeDesign,
}
impl Cache {
    pub fn new() -> Cache {
        Cache {
            word_layout_cache: HashMap::new(),
            symbol_layout_cache: HashMap::new(),
        }
    }

    fn render_word_part(&mut self, writer: &mut Writer, ctx: &DrawCtx, tag: Tag, font: Font, key: WordId, text: &str, part: WordPart) {
        let layout = self.word_layout_cache.entry((font, key, part))
            .or_insert_with(|| {
                let face = ctx.storage.get_font_face(font.font_face);
                Cache::build_word_layout(text, face, font.size.value)
            });
        let space = Glue::space(ctx.type_design.word_space);
        let width = layout.advance.x();
        let measure = ItemMeasure {
            left: FlexMeasure::zero(),
            content: FlexMeasure::fixed(Length::mm(width)),
            right: FlexMeasure::zero(),
            height: ctx.type_design.line_height
        };
        writer.item(space, space, measure, RenderItem::Word(key, part, font), tag);
    }

    fn render_word(&mut self, writer: &mut Writer, ctx: &DrawCtx, tag: Tag, key: WordId) {
        let dict = ctx.storage.get_dict(ctx.type_design.dictionary);
        let font = ctx.type_design.font;
        let word = ctx.storage.get_word(key);
        let space = Glue::space(ctx.type_design.word_space);
        let hyphen = ctx.storage.get_symbol(ctx.type_design.hyphen);
        let text = &word.text;

        writer.branch(|gen| {
            gen.add(|writer| self.render_word_part(writer, ctx, tag, font, key, text, WordPart::Full));
            dict.hyphenate(text, |index, before, after| {
                debug!("{} -> {}‐{} ({})", word.text, before, after, index);

                gen.add(|writer| {
                    self.render_word_part(writer, ctx, tag, font, key, before, WordPart::Before(index as u16));
                    
                    // hyphen
                    let width = Length::mm(self.symbol_layout(ctx.type_design.hyphen, &ctx.storage, font).advance.x());
                    writer.item(
                        Glue::None,
                        Glue::newline(),
                        ItemMeasure {
                            left: FlexMeasure::fixed(width * hyphen.overflow_left),
                            content: FlexMeasure::fixed(width),
                            right: FlexMeasure::fixed(width * hyphen.overflow_right),
                            height: ctx.type_design.line_height
                        },
                        RenderItem::Symbol(ctx.type_design.hyphen, font),
                        tag
                    );

                    self.render_word_part(writer, ctx, tag, font, key, after, WordPart::After(index as u16));
                });
            });
        });
    }

    fn render_symbol(&mut self, writer: &mut Writer, ctx: &DrawCtx, tag: Tag, key: SymbolId) {
        let symbol = ctx.storage.get_symbol(key);
        let font = ctx.type_design.font;
        let space = Glue::space(ctx.type_design.word_space);
        let width = Length::mm(self.symbol_layout(key, &ctx.storage, font).advance.x());
        writer.item(
            select(symbol.trailing, Glue::None, space),
            select(symbol.leading, Glue::None, space),
            ItemMeasure {
                left: FlexMeasure::fixed(width * symbol.overflow_left),
                content: FlexMeasure::fixed(width),
                right: FlexMeasure::fixed(width * symbol.overflow_right),
                height: ctx.type_design.line_height
            },
            RenderItem::Symbol(key, font),
            tag
        );
    }

    fn render_item(&mut self, writer: &mut Writer, ctx: &DrawCtx, tag: Tag, item: Item) {
        match item {
            Item::Word(key) => self.render_word(writer, ctx, tag, key),
            Item::Symbol(key) => self.render_symbol(writer, ctx, tag, key),
            Item::Object(key) => {
                let obj = ctx.storage.get_object(key);
                let (width, height) = obj.size(ctx);
                let measure = ItemMeasure {
                    left: FlexMeasure::zero(),
                    content: width,
                    right: FlexMeasure::zero(),
                    height
                };
                writer.item(Glue::any(), Glue::any(), measure, RenderItem::Object(key), tag);
            }
            Item::Sequence(key) => self.render_sequence(writer, ctx, key),
        }
    }
    fn render_sequence(&mut self, writer: &mut Writer, ctx: &DrawCtx, seq_id: SequenceId) {
        let weave = ctx.storage.get_weave(seq_id);
        let type_design = ctx.design.get_type_or_default(weave.typ());
        match type_design.display {
            Display::Block => writer.promote(Glue::newline()),
            Display::Paragraph(indent) => writer.space(Glue::newline(), Glue::hfill(), FlexMeasure::fixed(indent), false),
            _ => {}
        }
        let ctx = DrawCtx {
            type_design,
            .. *ctx
        };

        let measure = ItemMeasure {
            left: FlexMeasure::zero(),
            content: FlexMeasure::zero(),
            right: FlexMeasure::zero(),
            height: Length::zero()
        };
        writer.item(Glue::any(), Glue::None, measure, RenderItem::Empty, Tag::Start(seq_id));
        for (item_id, item) in weave.items() {
            self.render_item(writer, &ctx, Tag::Item(seq_id, item_id), item);
        }
        writer.item(Glue::None, Glue::any(), measure, RenderItem::Empty, Tag::End(seq_id));

        match ctx.type_design.display {
            Display::Block | Display::Paragraph(_) => writer.promote(Glue::hfill()),
            _ => {}
        }
    }

    pub fn layout(&mut self, storage: &Storage, design: &Design, target: &Target, root: SequenceId) -> Columns {
        let mut writer = Writer::new();
        let ctx = DrawCtx {
            storage,
            design,
            target,
            type_design: design.default()
        };
        self.render_sequence(&mut writer, &ctx, root);

        let stream = writer.finish();
        let layout = ColumnLayout::new(stream, target.content_box.width, target.content_box.height);
        layout.columns()
    }

    pub fn render_page(&mut self, storage: &Storage, target: &Target, design: &Design, column: Column) -> Page {
        let mut scene = Scene::new();
        scene.set_bounds(target.media_box.into());
        scene.set_view_box(target.trim_box.into());

        let page_style = scene.build_style(PathStyle {
            fill: Some((255,255,255,255)),
            stroke: Some(((0,0,0,255), 0.25)),
            fill_rule: FillRule::NonZero
        });
        let glyph_style = scene.build_style(PathStyle {
            fill: Some((0,0,0,255)),
            stroke: None,
            fill_rule: FillRule::NonZero
        });
        let mut pb = PathBuilder::new();
        pb.rect(target.trim_box.into());
        
        scene.draw_path(pb.into_outline(), &page_style);

        let ctx = DrawCtx {
            storage,
            design,
            target,
            type_design: design.default()
        };

        let mut line_indices = Vec::new();
        let mut positions = HashMap::new();
        let mut word_positions = HashMap::new();
        let content_box: RectF = target.content_box.into();
        for (y, line) in column {
            let mut line_items = Vec::new();
            for (x, size, item, tag) in line {
                let size: Vector2F = size.into();
                let p = content_box.origin() + Vector2F::new(x.value as f32, y.value as f32);
                let rect = RectF::new(p - Vector2F::new(0.0, size.y()), size);
                match item {
                    RenderItem::Word(key, part, font) => {
                        let layout = self.word_layout_cache.get(&(font, key, part)).unwrap();
                        let font = storage.get_font_face(font.font_face);
                        let outline = layout.render(font, Transform2F::from_translation(p));
                        scene.draw_path(outline, &glyph_style);

                        use std::collections::hash_map::Entry;
                        match (part, word_positions.entry(tag)) {
                            (WordPart::Full, Entry::Vacant(e)) => {
                                e.insert(RenderedWord::Full(rect));
                            }
                            (WordPart::Before(idx), Entry::Vacant(e)) => {
                                e.insert(RenderedWord::Before(rect, idx));
                            }
                            (WordPart::After(idx), Entry::Vacant(e)) => {
                                e.insert(RenderedWord::After(rect, idx));
                            }
                            (WordPart::After(idx), Entry::Occupied(mut e)) => {
                                match *e.get() {
                                    RenderedWord::Before(prev_rect, idx2) => {
                                        assert_eq!(idx, idx2);
                                        e.insert(RenderedWord::Both(prev_rect, rect, idx));
                                    }
                                    _ => panic!("invalid state")
                                }
                            },
                            _ => panic!()
                        }
                    }
                    RenderItem::Symbol(key, font) => {
                        let layout = self.symbol_layout(key, storage, font);
                        let font = storage.get_font_face(font.font_face);
                        let outline = layout.render(font, Transform2F::from_translation(p));
                        scene.draw_path(outline, &glyph_style);
                        line_items.push((p.x(), tag));
                        positions.insert(tag, rect);
                    }
                    RenderItem::Object(key) => {
                        storage.get_object(key).draw(&ctx, p, size.into(), &mut scene);
                        line_items.push((p.x(), tag));
                        positions.insert(tag, rect);
                    }
                    RenderItem::Empty => {
                        line_items.push((p.x(), tag));
                        positions.insert(tag, rect);
                    }
                };
            }
            line_indices.push((y.value + content_box.origin().y(), line_items));
        }

        Page { scene, items: line_indices, positions, word_positions }
    }

    pub fn get_position_on_page(&self, storage: &Storage, design: &Design, page: &Page, tag: Tag, byte_pos: usize) -> Option<Vector2F> {
        match storage.get_item(tag)? {
            Item::Word(key) => {
                let seq = storage.get_weave(tag.seq());
                let type_design = design.get_type_or_default(seq.typ());
                let word = storage.get_word(key);
                let face = storage.get_font_face(type_design.font.font_face);

                let (off, rect, part) = match *page.word_positions.get(&tag)? {
                    RenderedWord::Full(rect) => (0, rect, WordPart::Full),
                    RenderedWord::Before(rect, idx) => (0, rect, WordPart::Before(idx)),
                    RenderedWord::After(rect, idx) => (idx as usize, rect, WordPart::After(idx)),
                    RenderedWord::Both(rect, _, idx) if byte_pos < (idx as usize) => (0, rect, WordPart::Before(idx)),
                    RenderedWord::Both(_, rect, idx) => (idx as usize, rect, WordPart::After(idx))
                };
                if off > byte_pos {
                    return None;
                }
                let byte_pos = byte_pos - off;

                if byte_pos == 0 {
                    return Some(rect.lower_left());
                }
                let grapheme_indices = grapheme_indices(face, &word.text);
                let layout = self.word_layout_cache.get(&(type_design.font, key, part)).unwrap();

                let interpolate = |(idx_a, pos_a), (idx_b, pos_b), idx| -> Vector2F {
                    if idx == idx_b { return pos_b; }
                    let f = (idx - idx_a) as f32 / (idx_b - idx_a) as f32;
                    pos_a + (pos_b - pos_a).scale(f)
                };

                let mut last = (0, Vector2F::default());
                for (&n, &(_gid, offset)) in grapheme_indices.iter().zip(layout.glyphs.iter()) {
                    if n >= byte_pos {
                        return Some(rect.lower_left() + interpolate(last, (n, offset.vector,), byte_pos));
                    }
                    last = (n, offset.vector);
                }

                Some(rect.lower_left() + interpolate(last, (word.text.len(), layout.advance), byte_pos))
            }
            Item::Symbol(_) => {
                let &rect = page.positions.get(&tag)?;
                Some(rect.lower_right())
            }
            Item::Sequence(key) => {
                storage.get_first(key)
                    .and_then(|(first, _)| page.positions.get(&first))
                    .map(|rect| rect.lower_left())
            }
            _ => None
        }
    }
    pub fn find(&self, storage: &Storage, design: &Design, offset: f32, tag: Tag) -> Option<(Vector2F, usize)> {
        if let Some(Item::Word(key)) = storage.get_item(tag) {
            let seq = storage.get_weave(tag.seq());
            let type_design = design.get_type_or_default(seq.typ());
            let word = storage.get_word(key);
            let face = storage.get_font_face(type_design.font.font_face);
            let grapheme_indices = grapheme_indices(face, &word.text);

            let layout = self.word_layout_cache.get(&(type_design.font, key, WordPart::Full)).unwrap();

            if offset >= layout.advance.x() {
                return Some((layout.advance, word.text.len()));
            }
            for (&n, (_gid, tr)) in grapheme_indices.iter().rev().zip(layout.glyphs.iter().rev()) {
                if tr.vector.x() < offset {
                    println!("{}|{}", &word.text[0..n], &word.text[n..]);
                    return Some((tr.vector, n));
                }
            }
        }
        None
    }
    fn symbol_layout(&mut self, symbol: SymbolId, storage: &Storage, font: Font) -> &Layout {
        self.symbol_layout_cache.entry((font, symbol))
            .or_insert_with(|| {
                let text = &storage.get_symbol(symbol).text;
                let face = storage.get_font_face(font.font_face);
                Cache::build_word_layout(text, face, font.size.value)
            })
    }

    fn build_word_layout(text: &str, face: &FontFace, size: f32) -> Layout {
        let mut last_gid = None;
        let gids = build_gids(face, text);

        let transform = Transform2F::from_scale(Vector2F::splat(size))
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
}