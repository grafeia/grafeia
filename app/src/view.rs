
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_renderer::scene::Scene;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::options::{BuildOptions, RenderTransform};
use winit::{
    event::{Event, WindowEvent, DeviceEvent, KeyboardInput, ElementState, VirtualKeyCode, MouseButton, MouseScrollDelta, ModifiersState },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    dpi::{LogicalSize, LogicalPosition, PhysicalSize, PhysicalPosition},
};

use std::error::Error;
use serde::{Serialize, Deserialize};
use std::fs::File;

#[derive(Serialize, Deserialize)]
struct State {
    scale: f32,
    window_size: (f32, f32),
    view_center: (f32, f32)
}
fn try_read() -> Option<State> {
    let f = File::open(".view.json").ok()?;
    serde_json::from_reader(f).ok()
}
fn try_write(s: State) -> Option<()> {
    let f = File::create(".view.json").ok()?;
    serde_json::to_writer(f, &s).ok()
}

pub trait Interactive: 'static {
    fn scene(&mut self) -> Scene;
    fn char_input(&mut self, input: char) -> bool {
        false
    }
    fn keyboard_input(&mut self, state: ElementState, keycode: VirtualKeyCode) -> bool {
        false
    }
    fn mouse_input(&mut self, pos: Vector2F, state: ElementState) -> bool {
        false
    }
    fn exit(&mut self) {}
    fn title(&self) -> String { "A fantastic window!".into() }
}


pub fn show(mut item: impl Interactive) -> Result<(), Box<Error>> {
    info!("creating event loop");
    let event_loop = EventLoop::new();

    let mut scale = 1.0;

    let scene = item.scene();
    let view_box = scene.view_box();
    let mut view_center = view_box.origin() + view_box.size().scale(0.5);

    let mut window_size = view_box.size().scale(scale);

    if let Some(state) = try_read() {
        scale = state.scale;
        window_size = Vector2F::new(state.window_size.0, state.window_size.1);
        view_center = Vector2F::new(state.view_center.0, state.view_center.1);
    }

    info!("creating window");

    #[cfg(target_arch="wasm32")]
    let mut window = crate::webgl::WebGlWindow::new(&event_loop, "canvas");

    #[cfg(target_os="linux")]
    let mut window = crate::gl::GlWindow::new(&event_loop, item.title(), window_size);

    let mut dpi = window.scale_factor();
    let mut cursor_pos = Vector2F::default();
    let mut dragging = false;
    let mut modifiers = ModifiersState::empty();

    info!("entering the event loop");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            Event::RedrawRequested(_) => {
                let physical_size = window.framebuffer_size().to_f32();
                let scene = item.scene();
                let tr = Transform2F::from_translation(physical_size.scale(0.5)) *
                    Transform2F::from_scale(Vector2F::splat(dpi * scale)) *
                    Transform2F::from_translation(-view_center);
                
                let options = BuildOptions {
                    transform: RenderTransform::Transform2D(tr),
                    dilation: Vector2F::default(),
                    subpixel_aa_enabled: false
                };

                window.render(scene, options);
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::ModifiersChanged(new_modifiers) => {
                    modifiers = new_modifiers;
                },
                _ => {}
            }
            Event::WindowEvent { event, .. } =>  {
                let mut needs_redraw = false;
                let inverse_tr = |p: Vector2F| -> Vector2F {
                    Transform2F::from_translation(view_center) *
                    Transform2F::from_scale(Vector2F::splat(1.0 / (dpi * scale))) *
                    Transform2F::from_translation(window_size.scale(-0.5 * dpi)) *
                    p
                };

                match event {
                    WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size: &mut PhysicalSize { width, height } } => {
                        dpi = scale_factor as f32;
                        window.resize(Vector2F::new(width as _, height as _));
                        needs_redraw = true;
                    }
                    WindowEvent::Resized(PhysicalSize {width, height}) => {
                        window_size = Vector2F::new(width as f32, height as f32).scale(1.0 / dpi);
                        needs_redraw = true;
                    }
                    WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(keycode), .. }, ..  }
                        => needs_redraw |= item.keyboard_input(state, keycode),
                    WindowEvent::ReceivedCharacter(c) => needs_redraw |= item.char_input(c),
                    WindowEvent::CursorMoved { position: PhysicalPosition { x, y }, .. } => {
                        let new_pos = Vector2F::new(x as f32, y as f32);
                        let cursor_delta = new_pos - cursor_pos;
                        cursor_pos = new_pos;

                        if dragging {
                            view_center = view_center - cursor_delta.scale(1.0 / scale);
                            needs_redraw = true;
                        }
                    },
                    WindowEvent::MouseInput { button: MouseButton::Left, state, .. } => {
                        match (state, modifiers.shift()) {
                            (ElementState::Pressed, true) => dragging = true,
                            (ElementState::Released, _) if dragging => dragging = false,
                            _ => needs_redraw |= item.mouse_input(inverse_tr(cursor_pos), state)
                        }
                    },
                    WindowEvent::MouseWheel { delta, modifiers, .. } => {
                        let delta = match delta {
                            MouseScrollDelta::PixelDelta(LogicalPosition { x: dx, y: dy }) => Vector2F::new(dx as f32, dy as f32),
                            MouseScrollDelta::LineDelta(dx, dy) => Vector2F::new(dx as f32, -dy as f32).scale(10.)
                        };
                        if modifiers.ctrl() {
                            scale *= (-0.02 * delta.y()).exp();
                            needs_redraw = true;
                        } else {
                            view_center = view_center - delta.scale(1.0 / scale);
                            needs_redraw = true;
                        }
                    }
                    WindowEvent::CloseRequested => {
                        println!("The close button was pressed; stopping");
                        *control_flow = ControlFlow::Exit
                    },
                    _ => {}
                }
                if needs_redraw {
                    window.request_redraw();
                }
            }
            Event::LoopDestroyed => {
                let state = State {
                    scale,
                    window_size: (window_size.x(), window_size.y()),
                    view_center: (view_center.x(), view_center.y())
                };
                try_write(state);
                item.exit();
            }
            _ => {}
        }
    });
}

impl Interactive for Scene {
    fn scene(&mut self) -> Scene {
        self.clone()
    }
}
