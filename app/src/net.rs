use pathfinder_view::{Interactive, Context, ElementState, KeyEvent, KeyCode};
use pathfinder_renderer::scene::Scene;
use vector::{Vector, PathStyle, Surface, PathBuilder, Paint, FillRule, Rect};
use super::app::App;
use grafeia_core::{ClientCommand, ServerCommand, SiteId};

#[cfg(target_arch="wasm32")]
use crate::browser::Connection;

#[cfg(not(target_arch="wasm32"))]
use crate::desktop::Connection;

enum NetworkState {
    Connecting {
        site: Option<SiteId>,
    },
    Connected(App)
}
pub struct NetworkApp {
    state: NetworkState,
    conn: Option<Connection>
}
impl NetworkApp {
    pub fn new() -> Self {
        NetworkApp {
            state: NetworkState::Connecting { site: None },
            conn: None
        }
    }
}
impl Interactive for NetworkApp {
    fn title(&self) -> String {
        "γραφείο".into()
    }
    fn scene(&mut self, nr: usize) -> Scene {
        match self.state {
            NetworkState::Connected(ref mut app) => app.scene(nr),
            _ => {
                let mut scene = Scene::new();
                scene.set_view_box(Rect::new(Vector::default(), Vector::new(200., 200.)));
                let style = scene.build_style(PathStyle {
                    fill: None,
                    stroke: Some((Paint::Solid((0,0,200,255)), 10.)),
                    fill_rule: FillRule::NonZero
                });
                let mut pb = PathBuilder::new();
                pb.move_to(Vector::new(10.0, 80.0));
                pb.line_to(Vector::new(10.0, 10.0));
                pb.line_to(Vector::new(80.0, 10.0));
                scene.draw_path(pb.into_outline(), &style, None);
                scene
            }
        }
    }
    fn num_pages(&self) -> usize {
        match self.state {
            NetworkState::Connected(ref app) => app.num_pages(),
            _ => 1
        }
    }
    fn mouse_input(&mut self, ctx: &mut Context, page: usize, pos: Vector, state: ElementState) {
        match self.state {
            NetworkState::Connected(ref mut app) => app.mouse_input(ctx, page, pos, state),
            _ => {}
        }
    }

    fn keyboard_input(&mut self, ctx: &mut Context, event: &mut KeyEvent) {
        match self.state {
            NetworkState::Connected(ref mut app) => app.keyboard_input(ctx, event),
            _ => {}
        }
    }

    fn char_input(&mut self, ctx: &mut Context, c: char) {
        match self.state {
            NetworkState::Connected(ref mut app) => app.char_input(ctx, c),
            _ => {}
        }
    }
    fn exit(&mut self, _ctx: &mut Context) {
        /*
        match self.state {
            NetworkState::Connected(ref mut app) => app.exit(),
            _ => {}
        }
        */
    }
    fn event(&mut self, ctx: &mut Context, data: Vec<u8>) {
        let event = ServerCommand::<'static>::decode(&data).unwrap();

        match self.state {
            NetworkState::Connected(ref mut app) => match event {
                ServerCommand::Op(op) => {
                    app.op(op.into_owned());
                    ctx.update_scene();
                }
                _ => {}
            },
            NetworkState::Connecting { ref mut site } => match event {
                ServerCommand::Welcome(id) => {
                    *site = Some(id);
                    let conn = self.conn.as_mut().unwrap();
                    conn.send(ClientCommand::GetAll.encode());
                }
                ServerCommand::Document(state) => {
                    let site = site.expect("got Document before SiteId");
                    self.state = NetworkState::Connected(App::from_state(state, site));
                    ctx.update_scene();
                }
                _ => {}
            }
        }
    }
    fn init(&mut self, ctx: &mut Context) {
        let mut conn = Connection::init(ctx);
        conn.send(ClientCommand::Join.encode());
        self.conn = Some(conn);
    }
    fn idle(&mut self, _ctx: &mut Context) {
        match self.state {
            NetworkState::Connected(ref mut app) => {
                let conn = self.conn.as_mut().unwrap();
                for op in app.pending() {
                    conn.send(ClientCommand::Op(op).encode());
                }
            },
            _ => {}
        }
    }
}
