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
use futures::channel::mpsc::Sender;
use crate::build;

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

const VERSION: (u16, u16) = (0, 4);


// totally not a lazy hack to avoid implementing a (de)serializer
type AppStateBorrowed<'a> = (&'a Target, &'a Design, &'a LocalDocument);
type AppStateOwned = (Target, Design, LocalDocument);

pub struct App {
    target: Target,
    document: Document,
    design: Design,

    cache: Cache,
    pages: Vec<Page>,
    cursor: Option<Cursor>,
}
impl App {
    pub fn build() -> Self {
        build::build()
    }
    pub fn from_state(state: AppStateOwned, site: SiteId) -> Self {
        let (target, design, local_document) = state;
        let document = Document::from_local(local_document, site);
        let mut cache = Cache::new();

        let pages = cache.render(&target, &document, &design);
        App {
            cache,
            document,
            cursor: None,
            pages,
            target,
            design
        }
    }
    fn from_global(document: GlobalDocument, site: SiteId) -> Self {
        let (document, target, design) = Document::from_global(document, site);
        let mut cache = Cache::new();

        let pages = cache.render(&target, &document, &design);
        App {
            cache,
            document,
            cursor: None,
            pages,
            target,
            design
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
        let document = markdown::import_markdown(storage, &text);

        let target = build::default_target();

        App::from_state((target, design, document), SiteId(1))
    }

    #[cfg(feature="export_docx")]
    pub fn export_docx(&self) -> Vec<u8> {
        crate::export::docx::export_docx(&self.document, &self.design)
    }

    pub fn store(&self) {
        let state: AppStateBorrowed = (
            &self.target,
            &self.design,
            self.document.local()
        );
        store_data("app", &bincode::serialize(&(VERSION, state)).unwrap())
    }
    pub fn load_from(data: &[u8]) -> Option<Self> {
        info!("got {} bytes", data.len());
        let version: (u16, u16) = bincode::deserialize(data).ok()?;
        if version != VERSION {
            warn!("Ignoring data from an older version {}.{}. This is {}.{}.", version.0, version.1, VERSION.0, VERSION.1);
            return None;
        }
        let data = &data[bincode::serialized_size(&VERSION).unwrap() as usize ..];
        let state: AppStateOwned = bincode::deserialize(data).ok()?;
        info!("data decoded");

        Some(App::from_state(state, SiteId(1)))
    }
    pub fn load() -> Option<Self> {
        let data = load_data("app")?;
        Self::load_from(&data)
    }
    fn clean(&mut self) {
    }
    fn render(&mut self) { 
        self.pages = self.cache.render(&self.target, self.document.local(), &self.design);
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
        let seq = self.document.get_sequence(tag.seq);
        let type_key = seq.typ();
        
        match tag.pos {
            SequencePos::At(idx) => match (pos, &seq.items()[idx]) {
                (CursorPos::Before, &Item::Sequence(key)) => {
                    // end of sequence
                    let first_child = self.document.childen(key).filter_map(|tag| {
                        match self.document.get_item(tag).unwrap() {
                            &Item::Word(_) => Some(tag),
                            _ => None
                        }
                    }).next();
                    let first_child = dbg!(first_child).unwrap_or(Tag::end(key));
                    let rect = get!(self.pages[0].position(first_child));
                    let typ = self.design.get_type_or_default(type_key);
                    self.cursor = Some(Cursor {
                        tag,
                        pos,
                        page_pos: rect.lower_left() - Vector2F::new(0.5 * typ.word_space.width.value, 0.0),
                        type_key
                    });
                }
                (CursorPos::Before, _) => {
                    if let Some(page_pos) = self.document.get_previous_tag(tag).and_then(|left| self.cursor_between(left, tag)) {
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
                    self.cursor = self.cache.get_position_on_page(&self.design, &self.document, &self.pages[0], tag, text_pos)
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
                let left = get!(self.document.get_previous_tag(tag));
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

        if let Some(cursor) = self.cursor {
            assert!(cursor.page_pos.x().is_finite());
            assert!(cursor.page_pos.y().is_finite());
        }
    }
    fn text_op(&mut self, op: TextOp) -> Option<(Tag, CursorPos)> {
        let cursor = self.cursor?;

        match cursor.pos {
            CursorPos::Within(n) => {
                match self.document.get_item(cursor.tag)? {
                    &Item::Word(word_key) => {
                        let text = &self.document.get_word(word_key).text;

                        match op {
                            TextOp::DeleteGraphemeLeft if n > 0 => {
                                let new_pos = text[.. n].grapheme_indices(true).rev().next()
                                    .map(|(n, _)| n).unwrap_or(0);
                                let new_text = format!("{}{}", &text[.. new_pos], &text[n ..]);
                                if new_text.len() == 0 {
                                    let (tag, _) = self.document.remove(cursor.tag);
                                    return Some((tag, CursorPos::Before));
                                }
                                let new_item = self.document.add_word(&new_text);
                                self.document.replace(cursor.tag, new_item);
                                Some((cursor.tag, CursorPos::Within(new_pos)))
                            }
                            TextOp::DeleteGraphemeRight if n < text.len() => {
                                let new_pos = text[n ..].grapheme_indices(true).nth(1)
                                    .map(|(m, _)| n + m).unwrap_or(text.len());
                                let new_text = format!("{}{}", &text[.. n], &text[new_pos ..]);
                                if new_text.len() == 0 {
                                    self.document.remove(cursor.tag);
                                    return Some((cursor.tag, CursorPos::Before));
                                }
                                let new_item = self.document.add_word(&new_text);
                                self.document.replace(cursor.tag, new_item);

                                Some((cursor.tag, CursorPos::Within(n)))
                            }
                            TextOp::Insert(c) => {
                                let new_text = format!("{}{}{}", &text[.. n], c, &text[n ..]);
                                let new_item = self.document.add_word(&new_text);
                                self.document.replace(cursor.tag, new_item);

                                Some((cursor.tag, CursorPos::Within(n + c.len_utf8())))
                            }

                            // split, but only when within a word
                            TextOp::Split if n > 0 && n < text.len() => {
                                let left_text = text[.. n].to_owned();
                                let right_text = text[n ..].to_owned();
                                let left_item = self.document.add_word(&left_text);
                                let right_item = self.document.add_word(&right_text);
                                self.document.replace(cursor.tag, right_item);
                                self.document.insert(cursor.tag, left_item);

                                let tag = self.document.get_next_tag(cursor.tag)?;
                                Some((tag, CursorPos::Within(0)))
                            }

                            // place cursor before the word
                            TextOp::Split if n == 0 => {
                                Some((cursor.tag, CursorPos::Before))
                            }

                            // place cursor behind the word
                            TextOp::Split if n == text.len() => {
                                let tag = self.document.get_next_tag(cursor.tag)?;
                                Some((tag, CursorPos::Before))
                            }

                            TextOp::DeleteGraphemeLeft if n == 0 => {
                                // join with previous item … if possible
                                let left_tag = self.document.get_previous_tag(cursor.tag)?;
                                match self.document.get_item(left_tag)? {
                                    &Item::Word(left_word_key) => {
                                        let left_text = &self.document.get_word(left_word_key).text;
                                        let new_pos = left_text.len();
                                        let new_text = format!("{}{}", left_text, text);
                                        let new_item = self.document.add_word(&new_text);
                                        self.document.replace(left_tag, new_item);
                                        self.document.remove(cursor.tag);

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
                        let left_tag = self.document.get_previous_tag(cursor.tag)?;
                        self.document.remove(left_tag);
                        Some((left_tag, CursorPos::Before))
                    }
                    TextOp::DeleteItemRight => {
                        self.document.remove(cursor.tag);
                        Some((cursor.tag, CursorPos::Before))
                    }
                    TextOp::Insert(c) => {
                        let new_text = format!("{}", c);
                        let new_item = self.document.add_word(&new_text);
                        self.document.insert(cursor.tag, new_item);

                        Some((cursor.tag, CursorPos::Within(new_text.len())))
                    }
                    // place cursor at the end of the previous word
                    TextOp::DeleteGraphemeLeft => {
                        let left_tag = self.document.get_previous_tag(cursor.tag)?;
                        match self.document.get_item(left_tag)? {
                            &Item::Word(left_word_key) => {
                                let left_text = &self.document.get_word(left_word_key).text;
                                Some((left_tag, CursorPos::Within(left_text.len())))
                            }
                            _ => None
                        }
                    }
                    TextOp::NewSequence => {
                        let typ = self.document.get_sequence(cursor.tag.seq).typ();
                        let item = self.document.crate_seq(typ);
                        self.document.insert(cursor.tag, item);

                        Some((cursor.tag, CursorPos::Before))
                    }
                    _ => None
                }
            }
        }
    }
    fn cursor_op(&mut self, op: CursorOp) -> Option<(Tag, CursorPos)> {
        let cursor = self.cursor?;

        match (cursor.pos, self.document.get_item(cursor.tag)) {
            (CursorPos::Within(n), Some(&Item::Word(word_key))) => {
                let text = &self.document.get_word(word_key).text;
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
                        let right = self.document.get_next_tag(cursor.tag)?;
                        Some((right, CursorPos::Before))
                    }
                    CursorOp::ItemLeft => {
                        Some((cursor.tag, CursorPos::Before))
                    }
                    CursorOp::ItemRight => {
                        let right_tag = self.document.get_next_tag(cursor.tag)?;
                        Some((right_tag, CursorPos::Before))
                    }
                    _ => None
                }
            }
            (CursorPos::Before, _) => {
                match op {
                    CursorOp::GraphemeLeft => {
                        let left_tag = self.document.get_previous_tag(cursor.tag)?;
                        match self.document.get_item(left_tag)? {
                            &Item::Word(left_key) => {
                                let left_text = &self.document.get_word(left_key).text;
                                Some((left_tag, CursorPos::Within(left_text.len())))
                            },
                            _ => Some((left_tag, CursorPos::Before))
                        }
                    },
                    CursorOp::GraphemeRight => {
                        let right_tag = dbg!(self.document.get_next_tag(cursor.tag))?;
                        match self.document.get_item(right_tag)? {
                            &Item::Word(_) => Some((cursor.tag, CursorPos::Within(0))),
                            _ => Some((right_tag, CursorPos::Before))
                        }
                    },
                    CursorOp::ItemLeft => {
                        let left_tag = self.document.get_previous_tag(cursor.tag)?;
                        Some((left_tag, CursorPos::Before))
                    }
                    CursorOp::ItemRight => {
                        let right_tag = self.document.get_next_tag(cursor.tag)?;
                        Some((right_tag, CursorPos::Before))
                    }
                    _ => None
                }
            }
            _ => None
        }
    }

    fn op(&mut self, op: DocumentOp) -> bool {
        self.document.exec_op(op);
        self.render();
        true
    }

}

enum TextOp {
    Insert(char),
    Split,
    DeleteGraphemeLeft,
    DeleteGraphemeRight,
    DeleteItemLeft,
    DeleteItemRight,
    NewSequence,
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

            match self.document.get_item(cursor.tag) {
                Some(&Item::Sequence(key)) => {
                    let underline_style = scene.build_style(PathStyle {
                        fill: None,
                        stroke: Some(((0,200,0,255), 0.5))
                    });
        
                    let mut pb = PathBuilder::new();
                    for child in self.document.childen(key) {
                        if let Some(rect) = self.pages[0].position(child) {
                            pb.move_to(rect.lower_left());
                            pb.line_to(rect.lower_right());
                        }
                    }
                    scene.draw_path(pb.into_outline(), &underline_style);
                }
                Some(&Item::Object(_)) => {
                    let outline_style = scene.build_style(PathStyle {
                        fill: None,
                        stroke: Some(((200,0,0,255), 0.5))
                    });
        
                    if let Some(rect) = dbg!(self.pages[0].position(cursor.tag)) {
                        let mut pb = PathBuilder::new();
                        pb.rect(rect);
                        scene.draw_path(pb.into_outline(), &outline_style);
                    }
                }
                _ => {}
            }
        }

        scene
    }
    fn mouse_input(&mut self, pos: Vector2F, state: ElementState) -> bool {
        let old_cursor = self.cursor.take();

        dbg!(pos, state);
        if let Some((tag, word_pos)) = self.pages[0].find(pos) {
            let offset = pos.x() - word_pos.x();

            if let Some((word_offset, n, typ)) = self.cache.find(&self.design, &self.document, offset, tag) {
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
        if state == ElementState::Released {
            return false;
        }

        let (update, s) = match (keycode, modifiers.shift()) {
            (VirtualKeyCode::Right, false) => (false, self.cursor_op(CursorOp::GraphemeRight)),
            (VirtualKeyCode::Right, true) => (false, self.cursor_op(CursorOp::ItemRight)),
            (VirtualKeyCode::Left, false) => (false, self.cursor_op(CursorOp::GraphemeLeft)),
            (VirtualKeyCode::Left, true) => (false, self.cursor_op(CursorOp::ItemLeft)),
            (VirtualKeyCode::Back, false) => (true, self.text_op(TextOp::DeleteGraphemeLeft)),
            (VirtualKeyCode::Back, true) => (true, self.text_op(TextOp::DeleteItemLeft)),
            (VirtualKeyCode::Delete, false) => (true, self.text_op(TextOp::DeleteGraphemeRight)),
            (VirtualKeyCode::Delete, true) => (true, self.text_op(TextOp::DeleteItemRight)),
            (VirtualKeyCode::Return, false) => (true, self.text_op(TextOp::NewSequence)),
            _ => (false, None)
        };
        if update & s.is_some() {
            self.render();
        }
        if let Some((tag, pos)) = s {
            info!("new tag: {:?}", tag);
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

enum NetworkState {
    Connecting {
        site: Option<SiteId>,
    },
    Connected(App)
}
struct Connection {

}
pub struct NetworkApp {
    state: NetworkState,
    conn: Connection
}
impl NetworkApp {
    pub fn new() -> Self {
        NetworkApp {
            state: NetworkState::Connecting { site: None },
            conn: Connection {}
        }
    }
}
impl Connection {
    fn emit(&self, event: ClientCommand) {
        let data = bincode::serialize(&event).unwrap();
        self.platform_send(data);
    }
    #[cfg(target_arch = "wasm32")]
    fn platform_send(&self, data: Vec<u8>) {
        #[wasm_bindgen]
        extern {
            fn ws_send(data: &[u8]);
        }
        ws_send(&data);
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn platform_send(&self, data: Vec<u8>) {
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn platform_init(&mut self, emit: impl Fn(ServerCommand<'static>) + 'static) {
    }
    #[cfg(target_arch = "wasm32")]
    fn platform_init(&mut self, emit: impl Fn(ServerCommand<'static>) + 'static) {
        use js_sys::{Uint8Array, Function};
        use wasm_bindgen::{JsCast};
    
        #[wasm_bindgen]
        extern {
            fn set_ws_callback(cb: &Function);
        }
        let closure = Closure::wrap(Box::new(move |data: &Uint8Array| {
            let data = data.to_vec();
            match ServerCommand::<'static>::decode(&data) {
                Ok(val) => {
                    info!("recieved event");
                    emit(val);
                }
                Err(_) => warn!("invalid data")
            }
        }) as Box<Fn(&Uint8Array)>);
        set_ws_callback(closure.as_ref().unchecked_ref());
        closure.forget();
    }
}
impl Interactive for NetworkApp {
    type Event = ServerCommand<'static>;

    fn title(&self) -> String {
        "γραφείο".into()
    }
    fn scene(&mut self) -> Scene {
        match self.state {
            NetworkState::Connected(ref mut app) => app.scene(),
            _ => {
                let mut scene = Scene::new();
                let style = scene.build_style(PathStyle {
                    fill: None,
                    stroke: Some(((0,0,200,255), 10.))
                });
                let mut pb = PathBuilder::new();
                pb.move_to(Vector2F::new(0.0, 100.0));
                pb.line_to(Vector2F::new(0.0, 0.0));
                pb.line_to(Vector2F::new(50.0, 0.0));
                scene.draw_path(pb.into_outline(), &style);
                scene
            }
        }
    }
    fn mouse_input(&mut self, pos: Vector2F, state: ElementState) -> bool {
        match self.state {
            NetworkState::Connected(ref mut app) => app.mouse_input(pos, state),
            _ => false
        }
    }

    fn keyboard_input(&mut self, state: ElementState, keycode: VirtualKeyCode, modifiers: ModifiersState) -> bool {
        match self.state {
            NetworkState::Connected(ref mut app) => app.keyboard_input(state, keycode, modifiers),
            _ => false
        }
    }

    fn char_input(&mut self, c: char) -> bool {
        match self.state {
            NetworkState::Connected(ref mut app) => app.char_input(c),
            _ => false
        }
    }
    fn exit(&mut self) {
        match self.state {
            NetworkState::Connected(ref mut app) => app.exit(),
            _ => {}
        }
    }
    fn save_state(&self, state: State) {
        store_data("view", &bincode::serialize(&state).unwrap())
    }
    fn load_state(&self) -> Option<State> {
        let data = load_data("view")?;
        bincode::deserialize(&data).ok()
    }
    fn event(&mut self, event: Self::Event) -> bool {
        match event {
            ServerCommand::Welcome(site) => info!("-> Welcome({:?})", site),
            ServerCommand::Document(_) => info!("-> Document"),
            ServerCommand::Op(ref op) => info!("-> Op({:?})", op),
        }

        match self.state {
            NetworkState::Connected(ref mut app) => match event {
                ServerCommand::Op(op) => app.op(op.into_owned()),
                _ => false
            },
            NetworkState::Connecting { ref mut site } => match event {
                ServerCommand::Welcome(id) => {
                    *site = Some(id);
                    self.conn.emit(ClientCommand::GetAll);
                    false
                }
                ServerCommand::Document(document) => {
                    let site = site.expect("got Document before SiteId");
                    self.state = NetworkState::Connected(App::from_global(document.into_owned(), site));
                    true
                }
                _ => false
            }
        }
    }
    fn init(&mut self, emit: impl Fn(Self::Event) + 'static) {
        self.conn.platform_init(emit);
        self.conn.emit(ClientCommand::Join);
    }
    fn idle(&mut self) {
        match self.state {
            NetworkState::Connected(ref mut app) => {
                for op in app.document.drain_pending() {
                    self.conn.emit(ClientCommand::Op(op));
                }
            },
            _ => {}
        }
    }
}