use grafeia_core::{
    *,
    units::*,
    builder::ContentBuilder,
    draw::{Cache, Page},
    layout::FlexMeasure,
    Color, Display
};
use pathfinder_renderer::scene::Scene;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_view::{Interactive, State};
use winit::event::{ElementState, VirtualKeyCode, ModifiersState};
use vector::{PathBuilder, PathStyle, Surface};
use unicode_segmentation::UnicodeSegmentation;
use serde::{Serialize, Deserialize};
use unicode_categories::UnicodeCategories;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
fn store_data(key: &str, data: &[u8]) {
    std::fs::write(&format!(".{}.data", key), data).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
fn load_data(key: &str) -> Option<Vec<u8>> {
    std::fs::read(&format!(".{}.data", key)).ok()
}

#[cfg(target_arch = "wasm32")]
fn store_data(key: &str, data: &[u8]) {
    let encoded = base64::encode(data);
    web_sys::window().unwrap()
        .local_storage().unwrap().unwrap()
        .set_item(key, &encoded).unwrap();
}

#[cfg(target_arch = "wasm32")]
fn load_data(key: &str) -> Option<Vec<u8>> {
    let encoded = web_sys::window().unwrap()
        .local_storage().unwrap().unwrap()
        .get_item(key).unwrap()?;
    
    base64::decode(&encoded).ok()
}

macro_rules! get {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => return
        }
    };
}

const VERSION: (u16, u16) = (0, 3);

fn default_target() -> Target {
    Target {
        description: "test target".into(),
        content_box: Rect {
            left: Length::mm(10.),
            width: Length::mm(150.),
            top: Length::mm(10.),
            height: Length::mm(220.)
        },
        media_box: Rect {
            left: Length::mm(-3.),
            width: Length::mm(176.),
            top: Length::mm(-3.),
            height: Length::mm(246.)
        },
        trim_box: Rect {
            left: Length::mm(0.),
            width: Length::mm(170.),
            top: Length::mm(0.),
            height: Length::mm(240.)
        },
        page_color: Color
    }
}

#[derive(Serialize, Deserialize)]
pub struct App {
    storage: Storage,
    target: Target,
    document: Document,
    design: Design,

    #[serde(skip)]
    cache: Cache,

    #[serde(skip)]
    pages: Vec<Page>,

    #[serde(skip)]
    cursor: Option<Cursor>
}
impl App {
    pub fn build() -> Self {
        info!("App::build()");

        let mut storage = Storage::new();
        let document = ContentBuilder::new(&mut storage)
            .chapter().word("Test").finish()
            .paragraph()
                .word("The")
                .word("distilled")
                .word("spirit")
                .word("of")
                .word("Garamond")
                .finish()
            .paragraph()
                .word("The")
                .word("ffine")
                .word("fish")
                .finish()
            .paragraph()
                .word("Written")
                .word("in")                
                .object(Object::Svg(SvgObject::new(Size::FitTextHeight, include_bytes!("../../data/rust_logo.svg")[..].into())))
                .word("using")
                .object(Object::Svg(SvgObject::new(Size::FitTextHeight, include_bytes!("../../data/pathfinder_logo.svg")[..].into())))
                .finish()
            .object(Object::Svg(SvgObject::new(Size::FitWidth, include_bytes!("../../data/Ghostscript_Tiger.svg")[..].into())))
            .finish();
        
        

        info!("reading font");
        let font_face = storage.insert_font_face(
            Vec::from(&include_bytes!("../../data/Cormorant-Regular.ttf")[..]).into()
        );

        info!("done reading font");

        let default = TypeDesign {
            display:   Display::Inline,
            font:           Font {
                font_face,
                size:  Length::mm(4.0)
            },
            word_space: FlexMeasure {
                height:  Length::zero(),
                shrink:  Length::mm(1.0),
                width:   Length::mm(2.0),
                stretch: Length::mm(3.0)
            },
            line_height: Length::mm(5.0)
        };

        
        let mut design = Design::new("default design".into(), default);
        design.set_type(
            storage.find_type("chapter").unwrap(),
            TypeDesign {
                display:        Display::Block,
                font:           Font {
                    font_face,
                    size:  Length::mm(8.0)
                },
                word_space: FlexMeasure {
                    height:  Length::zero(),
                    shrink:  Length::mm(2.0),
                    width:   Length::mm(4.0),
                    stretch: Length::mm(6.0)
                },
                line_height: Length::mm(10.0)
            }
        );
        design.set_type(
            storage.find_type("paragraph").unwrap(),
            TypeDesign {
                display:        Display::Paragraph(Length::mm(10.0)),
                font:           Font {
                    font_face,
                    size:  Length::mm(4.0)
                },
                word_space: FlexMeasure {
                    height:  Length::zero(),
                    shrink:  Length::mm(1.0),
                    width:   Length::mm(2.0),
                    stretch: Length::mm(3.0)
                },
                line_height: Length::mm(5.0)
            }
        );

        let target = default_target();
        let mut cache = Cache::new();
        info!("rendering document");
        let pages = cache.render(&storage, &target, &document, &design);
        info!("App ready");

        App {
            cache,
            storage,
            target,
            document,
            design,
            pages,
            cursor: None
        }
    }

    #[cfg(feature="import_markdown")]
    pub fn import_markdown(file: &str) -> Self {
        let mut storage = Storage::new();
        let data = std::fs::read(file).unwrap();
        let text = String::from_utf8(data).unwrap();

        use crate::import::markdown;
        markdown::define_types(&mut storage);
        let design = markdown::markdown_design(&mut storage);
        let document = markdown::import_markdown(&mut storage, &text);

        let target = default_target();
        let mut cache = Cache::new();
        info!("rendering document");
        let pages = cache.render(&storage, &target, &document, &design);
        info!("App ready");

        App {
            cache,
            storage,
            target,
            document,
            design,
            pages,
            cursor: None
        }
    }

    #[cfg(feature="export_docx")]
    pub fn export_docx(&self) -> Vec<u8> {
        crate::export::docx::export_docx(&self.storage, &self.document, &self.design)
    }

    pub fn store(&self) {
        store_data("app", &bincode::serialize(&(VERSION, self)).unwrap())
    }
    pub fn load_from(data: &[u8]) -> Option<Self> {
        info!("got {} bytes", data.len());
        let version: (u16, u16) = bincode::deserialize(data).ok()?;
        if version != VERSION {
            warn!("Ignoring data from an older version {}.{}. This is {}.{}.", version.0, version.1, VERSION.0, VERSION.1);
            return None;
        }
        let data = &data[bincode::serialized_size(&VERSION).unwrap() as usize ..];
        let mut app: Self = bincode::deserialize(data).ok()?;
        info!("data decoded");
        app.render();
        Some(app)
    }
    pub fn load() -> Option<Self> {
        let data = load_data("app")?;
        Self::load_from(&data)
    }
    fn clean(&mut self) {
    }
    fn render(&mut self) {
        self.pages = self.cache.render(&self.storage, &self.target, &self.document, &self.design);
    }
    fn cursor_between(&mut self, left_tag: Tag, tag: Tag) -> Option<Vector2F> {
        let left_rect = self.pages[0].position(left_tag)?;
        let rect = self.pages[0].position(tag)?;
        let l = left_rect.lower_right();
        let r = rect.lower_left();
        if l.y() == r.y() {
            Some((l + r).scale(0.5))
        } else {
            None
        }
    }
    fn set_cursor_to(&mut self, tag: Tag, pos: CursorPos) {
        let seq = self.storage.get_sequence(tag.seq);
        let type_key = seq.typ();
        
        match tag.pos {
            SequencePos::At(idx) => match (pos, &seq.items()[idx]) {
                (CursorPos::Before, &Item::Sequence(key)) => {
                    // end of sequence
                    let seq = self.storage.get_sequence(key);
                    let right = get!(self.document.get_next_tag(&self.storage, tag));
                    let rect = get!(self.pages[0].position(right));
                    let typ = self.design.get_type_or_default(type_key);
                    self.cursor = Some(Cursor {
                        tag,
                        pos,
                        page_pos: rect.lower_right() - Vector2F::new(0.5 * typ.word_space.width.value, 0.0),
                        type_key
                    })
                }
                (CursorPos::Before, _) => {
                    if let Some(page_pos) = self.document.get_previous_tag(&self.storage, tag).and_then(|left| self.cursor_between(left, tag)) {
                        self.cursor = Some(Cursor {
                            tag,
                            pos,
                            page_pos,
                            type_key
                        });
                    } else if let Some(rect) = self.pages[0].position(tag) {
                        let typ = self.design.get_type_or_default(type_key);
                        self.cursor = Some(Cursor {
                            tag,
                            pos,
                            page_pos: rect.lower_left() - Vector2F::new(0.5 * typ.word_space.width.value, 0.0),
                            type_key
                        });
                    } else {
                        self.cursor = None;
                    }
                }
                (CursorPos::Within(text_pos), &Item::Word(_)) => {
                    self.cursor = self.cache.get_position_on_page(&self.storage, &self.design, &self.document, &self.pages[0], tag, text_pos)
                    .map(|page_pos| Cursor {
                        tag,
                        pos,
                        page_pos,
                        type_key
                    });
                }
                _ => {}
            }
            SequencePos::End => {
                // end of sequence
                let left = get!(self.document.get_previous_tag(&self.storage, tag));
                let rect = get!(self.pages[0].position(left));
                let typ = self.design.get_type_or_default(type_key);
                self.cursor = Some(Cursor {
                    tag,
                    pos,
                    page_pos: rect.lower_right() + Vector2F::new(0.5 * typ.word_space.width.value, 0.0),
                    type_key
                })
            }
        }

        dbg!(self.cursor);
        if let Some(cursor) = self.cursor {
            assert!(cursor.page_pos.x().is_finite());
            assert!(cursor.page_pos.y().is_finite());
        }
    }
    fn text_op(&mut self, op: TextOp) -> Option<(Tag, CursorPos)> {
        let cursor = self.cursor?;

        match cursor.pos {
            CursorPos::Within(n) => {
                match self.storage.get_item(cursor.tag)? {
                    &Item::Word(word_key) => {
                        let text = &self.storage.get_word(word_key).text;
                        dbg!(text, n);

                        match op {
                            TextOp::DeleteGraphemeLeft if n > 0 => {
                                let new_pos = text[.. n].grapheme_indices(true).rev().next()
                                    .map(|(n, _)| n).unwrap_or(0);
                                let new_text = format!("{}{}", &text[.. new_pos], &text[n ..]);
                                if new_text.len() == 0 {
                                    self.document.remove(&mut self.storage, cursor.tag);
                                    return Some((cursor.tag, CursorPos::Before));
                                }
                                let new_item = Item::Word(self.storage.insert_word(&new_text));
                                self.document.replace(&mut self.storage, cursor.tag, new_item);
                                Some((cursor.tag, CursorPos::Within(new_pos)))
                            }
                            TextOp::DeleteGraphemeRight if n < text.len() => {
                                let new_pos = text[n ..].grapheme_indices(true).nth(1)
                                    .map(|(m, _)| n + m).unwrap_or(text.len());
                                let new_text = format!("{}{}", &text[.. n], &text[new_pos ..]);
                                if new_text.len() == 0 {
                                    self.document.remove(&mut self.storage, cursor.tag);
                                    return Some((cursor.tag, CursorPos::Before));
                                }
                                let new_item = Item::Word(self.storage.insert_word(&new_text));
                                self.document.replace(&mut self.storage, cursor.tag, new_item);

                                Some((cursor.tag, CursorPos::Within(n)))
                            }
                            TextOp::Insert(c) => {
                                let new_text = format!("{}{}{}", &text[.. n], c, &text[n ..]);
                                let new_item = Item::Word(self.storage.insert_word(&new_text));
                                self.document.replace(&mut self.storage, cursor.tag, new_item);

                                Some((cursor.tag, CursorPos::Within(n + c.len_utf8())))
                            }

                            // split, but only when within a word
                            TextOp::Split if n > 0 && n < text.len() => {
                                let left_text = text[.. n].to_owned();
                                let right_text = text[n ..].to_owned();
                                let left_item = Item::Word(self.storage.insert_word(&left_text));
                                let right_item = Item::Word(self.storage.insert_word(&right_text));
                                self.document.replace(&mut self.storage, cursor.tag, right_item);
                                self.document.insert(&mut self.storage, cursor.tag, left_item);

                                let tag = self.document.get_next_tag(&self.storage, cursor.tag)?;
                                Some((tag, CursorPos::Within(0)))
                            }

                            // place cursor before the word
                            TextOp::Split if n == 0 => {
                                Some((cursor.tag, CursorPos::Before))
                            }

                            // place cursor behind the word
                            TextOp::Split if n == text.len() => {
                                let tag = self.document.get_next_tag(&self.storage, cursor.tag)?;
                                Some((tag, CursorPos::Before))
                            }

                            TextOp::DeleteGraphemeLeft if n == 0 => {
                                // join with previous item … if possible
                                let left_tag = self.document.get_previous_tag(&self.storage, cursor.tag)?;
                                match self.storage.get_item(left_tag)? {
                                    &Item::Word(left_word_key) => {
                                        let left_text = &self.storage.get_word(left_word_key).text;
                                        let new_pos = left_text.len();
                                        let new_text = format!("{}{}", left_text, text);
                                        let new_item = Item::Word(self.storage.insert_word(&new_text));
                                        self.document.replace(&mut self.storage, left_tag, new_item);
                                        self.document.remove(&mut self.storage, cursor.tag);

                                        Some((left_tag, CursorPos::Within(new_pos)))
                                    }
                                    _ => None
                                }
                            }
                            _ => None
                        }
                    }
                    _ => None
                }
            }
            CursorPos::Before => {
                match op {
                    TextOp::DeleteItemLeft => {
                        let left_tag = self.document.get_previous_tag(&self.storage, cursor.tag)?;
                        self.document.remove(&mut self.storage, left_tag);
                        Some((left_tag, CursorPos::Before))
                    }
                    TextOp::DeleteItemRight => {
                        self.document.remove(&mut self.storage, cursor.tag);
                        Some((cursor.tag, CursorPos::Before))
                    }
                    TextOp::Insert(c) => {
                        let new_text = format!("{}", c);
                        let new_item = Item::Word(self.storage.insert_word(&new_text));
                        self.document.insert(&mut self.storage, cursor.tag, new_item);

                        Some((cursor.tag, CursorPos::Within(new_text.len())))
                    }
                    // place cursor at the end of the previous word
                    TextOp::DeleteGraphemeLeft => {
                        let left_tag = self.document.get_previous_tag(&self.storage, cursor.tag)?;
                        match self.storage.get_item(left_tag)? {
                            &Item::Word(left_word_key) => {
                                let left_text = &self.storage.get_word(left_word_key).text;
                                Some((left_tag, CursorPos::Within(left_text.len())))
                            }
                            _ => None
                        }
                    }
                    _ => None
                }
            }
        }
    }
    fn cursor_op(&mut self, op: CursorOp) -> Option<(Tag, CursorPos)> {
        let cursor = self.cursor?;

        match (cursor.pos, self.storage.get_item(cursor.tag)) {
            (CursorPos::Within(n), Some(&Item::Word(word_key))) => {
                let text = &self.storage.get_word(word_key).text;
                match op {
                    CursorOp::GraphemeRight if n < text.len() => {
                        let pos = text[n ..].grapheme_indices(true).nth(1)
                            .map(|(m, _)| n + m).unwrap_or(text.len());
                        Some((cursor.tag, CursorPos::Within(pos)))
                    }
                    CursorOp::GraphemeLeft if n > 0 => {
                        let pos = text[.. n].grapheme_indices(true).rev().next()
                            .map(|(n, _)| n).unwrap_or(0);
                        Some((cursor.tag, CursorPos::Within(pos)))
                    }
                    CursorOp::GraphemeLeft => {
                        Some((cursor.tag, CursorPos::Before))
                    }
                    CursorOp::GraphemeRight => {
                        let right = self.document.get_next_tag(&self.storage, cursor.tag)?;
                        Some((right, CursorPos::Before))
                    }
                    CursorOp::ItemLeft => {
                        Some((cursor.tag, CursorPos::Before))
                    }
                    CursorOp::ItemRight => {
                        let right_tag = self.document.get_next_tag(&self.storage, cursor.tag)?;
                        Some((right_tag, CursorPos::Before))
                    }
                    _ => None
                }
            }
            (CursorPos::Before, _) => {
                match op {
                    CursorOp::GraphemeLeft => {
                        let left_tag = self.document.get_previous_tag(&self.storage, cursor.tag)?;
                        match self.storage.get_item(left_tag)? {
                            &Item::Word(left_key) => {
                                let left_text = &self.storage.get_word(left_key).text;
                                Some((left_tag, CursorPos::Within(left_text.len())))
                            },
                            _ => Some((left_tag, CursorPos::Before))
                        }
                    },
                    CursorOp::GraphemeRight => {
                        let right_tag = self.document.get_next_tag(&self.storage, cursor.tag)?;
                        match self.storage.get_item(right_tag)? {
                            &Item::Word(_) => Some((cursor.tag, CursorPos::Within(0))),
                            _ => Some((right_tag, CursorPos::Before))
                        }
                    },
                    CursorOp::ItemLeft => {
                        let left_tag = self.document.get_previous_tag(&self.storage, cursor.tag)?;
                        Some((left_tag, CursorPos::Before))
                    }
                    CursorOp::ItemRight => {
                        let right_tag = self.document.get_next_tag(&self.storage, cursor.tag)?;
                        Some((right_tag, CursorPos::Before))
                    }
                    _ => None
                }
            }
            _ => None
        }
    }
}

enum TextOp {
    Insert(char),
    Split,
    DeleteGraphemeLeft,
    DeleteGraphemeRight,
    DeleteItemLeft,
    DeleteItemRight,
}
enum CursorOp {
    GraphemeLeft,
    GraphemeRight,
    ItemRight,
    ItemLeft,
}


#[derive(PartialEq, Copy, Clone, Debug)]
enum CursorPos {
    Before,
    Within(usize)
}

#[derive(PartialEq, Copy, Clone, Debug)]
struct Cursor {
    tag: Tag,   // which item
    pos: CursorPos, // between this and the following
    page_pos: Vector2F,
    type_key: TypeKey
}

impl Interactive for App {
    fn title(&self) -> String {
        "γραφείο".into()
    }
    fn scene(&mut self) -> Scene {
        let mut scene = self.pages[0].scene().clone();
        if let Some(ref cursor) = self.cursor {
            let type_design = self.design.get_type_or_default(cursor.type_key);
            let style = scene.build_style(PathStyle {
                fill: None,
                stroke: Some(((0,0,200,255), 0.1 * type_design.font.size.value))
            });
            let mut pb = PathBuilder::new();
            pb.move_to(cursor.page_pos);
            pb.line_to(cursor.page_pos - Vector2F::new(0.0, type_design.font.size.value));
            
            scene.draw_path(pb.into_outline(), &style);
        }

        scene
    }
    fn mouse_input(&mut self, pos: Vector2F, state: ElementState) -> bool {
        info!("mouse input at {:?}, state = {:?}", pos, state);
        let old_cursor = self.cursor.take();

        dbg!(pos, state);
        if let Some((tag, word_pos)) = self.pages[0].find(pos) {
            let offset = pos.x() - word_pos.x();

            if let Some((word_offset, n, typ)) = self.cache.find(&self.storage, &self.design, &self.document, offset, tag) {
                self.cursor = Some(Cursor {
                    tag,
                    page_pos: word_offset + word_pos,
                    pos: CursorPos::Within(n),
                    type_key: typ
                });
            } else {
                self.set_cursor_to(tag, CursorPos::Before);
            }
        }

        self.cursor != old_cursor
    }

    fn keyboard_input(&mut self, state: ElementState, keycode: VirtualKeyCode, modifiers: ModifiersState) -> bool {
        info!("keyboard input keycode = {:?}, state = {:?}", keycode, state);
        let (update, s) = match (state, keycode, modifiers.shift()) {
            (ElementState::Pressed, VirtualKeyCode::Right, false) => (false, self.cursor_op(CursorOp::GraphemeRight)),
            (ElementState::Pressed, VirtualKeyCode::Right, true) => (false, self.cursor_op(CursorOp::ItemRight)),
            (ElementState::Pressed, VirtualKeyCode::Left, false) => (false, self.cursor_op(CursorOp::GraphemeLeft)),
            (ElementState::Pressed, VirtualKeyCode::Left, true) => (false, self.cursor_op(CursorOp::ItemLeft)),
            (ElementState::Pressed, VirtualKeyCode::Back, false) => (true, self.text_op(TextOp::DeleteGraphemeLeft)),
            (ElementState::Pressed, VirtualKeyCode::Back, true) => (true, self.text_op(TextOp::DeleteItemLeft)),
            (ElementState::Pressed, VirtualKeyCode::Delete, false) => (true, self.text_op(TextOp::DeleteGraphemeRight)),
            (ElementState::Pressed, VirtualKeyCode::Delete, true) => (true, self.text_op(TextOp::DeleteItemRight)),
            _ => (false, None)
        };
        if update & s.is_some() {
            self.render();
        }
        if let Some((tag, pos)) = s {
            self.set_cursor_to(tag, pos);
            true
        } else {
            false
        }
    }

    fn char_input(&mut self, c: char) -> bool {
        let s = match c {
            // backspace
            ' ' => self.text_op(TextOp::Split),
            c if c.is_letter() => self.text_op(TextOp::Insert(c)),
            _ => None
        };
        if let Some((tag, pos)) = s {
            self.render();
            self.set_cursor_to(tag, pos);
            true
        } else {
            false
        }
    }
    fn exit(&mut self) {
        self.clean();
        self.store()
    }
    fn save_state(&self, state: State) {
        store_data("view", &bincode::serialize(&state).unwrap())
    }
    fn load_state(&self) -> Option<State> {
        let data = load_data("view")?;
        bincode::deserialize(&data).ok()
    }
}
