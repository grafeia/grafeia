use grafeia_core::{
    content::*,
    units::*,
    builder::ContentBuilder,
    draw::{Cache, Page},
    layout::FlexMeasure,
    Color, Display
};
use font;
use pathfinder_renderer::scene::Scene;
use pathfinder_geometry::vector::Vector2F;
use crate::view::Interactive;
use winit::event::{ElementState, VirtualKeyCode};
use vector::{PathBuilder, PathStyle, Surface};
use unicode_segmentation::UnicodeSegmentation;
use serde::{Serialize, Deserialize};
use std::fs::File;

#[derive(Serialize, Deserialize)]
pub struct App {
    storage: Storage,
    target: Target,
    document: Sequence,
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
            .finish();
        
        let target = Target {
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
        };

        info!("reading font");
        let font_face = storage.insert_font_face(FontFace::from_path("/home/sebk/Rust/font/fonts/Cormorant/Cormorant-Regular.ttf").unwrap());

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
    pub fn store(&self) {
        bincode::serialize_into(File::create("app.data").unwrap(), self).unwrap()
    }
    pub fn load() -> Option<Self> {
        let file = File::open("app.data").ok()?;
        let mut app: Self = bincode::deserialize_from(file).ok()?;
        app.render();
        Some(app)
    }
    fn render(&mut self) {
        self.pages = self.cache.render(&self.storage, &self.target, &self.document, &self.design);
    }
    fn set_cursor_to(&mut self, tag: Tag, text_pos: usize) {
        self.cursor = self.cache.get_position_on_page(&self.storage, &self.design, &self.document, &self.pages[0], tag, text_pos)
            .map(|(page_pos, type_key)| Cursor {
                tag,
                text_pos,
                page_pos,
                type_key
            });
    }
}

#[derive(PartialEq, Copy, Clone)]
struct Cursor {
    tag: Tag,   // which item
    text_pos: usize, // which byte in the item (if applicable)
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
        let old_cursor = self.cursor.take();

        dbg!(pos, state);
        if let Some((tag, word_pos)) = self.pages[0].find(pos) {
            let item = self.document.find(tag);
            println!("clicked on {:?}", item);
            let offset = pos.x() - word_pos.x();

            self.cursor = self.cache.find(&self.storage, &self.design, &self.document, offset, tag)
                .map(|(word_offset, n, typ)| Cursor {
                    tag,
                    page_pos: word_offset + word_pos,
                    text_pos: n,
                    type_key: typ
                });
        }

        self.cursor != old_cursor
    }

    fn keyboard_input(&mut self, state: ElementState, keycode: VirtualKeyCode) -> bool {
        match (state, keycode) {
            (ElementState::Pressed, VirtualKeyCode::Right) => {
                if let Some(cursor) = self.cursor {
                    match self.document.find(cursor.tag) {
                        Some((_, &Item::Word(word_key))) => {
                            let text = &self.storage.get_word(word_key).text;
                            let pos = match text[cursor.text_pos ..].grapheme_indices(true).nth(1) {
                                Some((offset, _)) => cursor.text_pos + offset,
                                None => text.len()
                            };
                            self.set_cursor_to(cursor.tag, pos);
                            return true;
                        }
                        _ => {}
                    }
                }
            },
            (ElementState::Pressed, VirtualKeyCode::Left) => {
                if let Some(cursor) = self.cursor {
                    match self.document.find(cursor.tag) {
                        Some((_, &Item::Word(word_key))) => {
                            let text = &self.storage.get_word(word_key).text;
                            if let Some((pos, _)) = text[.. cursor.text_pos].grapheme_indices(true).rev().next() {
                                self.set_cursor_to(cursor.tag, pos);
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
            },
            _ => {}
        }
        false
    }

    fn char_input(&mut self, c: char) -> bool {
        if let Some(cursor) = self.cursor.take() {
            match self.document.find(cursor.tag) {
                Some((_, &Item::Word(word_key))) => {
                    dbg!(&self.document);
                    let old_text = &self.storage.get_word(word_key).text;

                    let (new_text, text_pos) = match c {
                        // backspace
                        '\u{8}' if cursor.text_pos > 0 => {
                            let new_pos = old_text[.. cursor.text_pos].grapheme_indices(true).rev().next().unwrap().0;
                            let new_text = format!("{}{}", &old_text[.. new_pos], &old_text[cursor.text_pos ..]);
                            (new_text, new_pos)
                        },
                        '\u{8}' => return false,
                        ' ' => return false,
                        _ => {
                            let new_text = format!("{}{}{}", &old_text[.. cursor.text_pos], c, &old_text[cursor.text_pos ..]);
                            (new_text, cursor.text_pos + c.len_utf8())
                        }
                    };
                    
                    let new_item = Item::Word(self.storage.insert_word(&new_text));
                    self.document.replace(cursor.tag, new_item);

                    self.render();

                    self.cursor = self.cache.get_position_on_page(&self.storage, &self.design, &self.document, &self.pages[0], cursor.tag, text_pos)
                        .map(|(page_pos, type_key)| Cursor {
                            tag: cursor.tag,
                            text_pos,
                            page_pos,
                            type_key
                        });

                    return true;
                },
                _ => {}
            }
        }
        false
    }
    fn exit(&mut self) {
        self.store()
    }
}
