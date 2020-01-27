use winit::platform::web::{WindowBuilderExtWebSys};

use pathfinder_webgl::WebGlDevice;
use pathfinder_renderer::{
    gpu::{
        options::{DestFramebuffer, RendererOptions},
        renderer::Renderer
    },
    gpu_data::RenderCommand,
    scene::Scene,
    options::{BuildOptions, RenderTransform, RenderCommandListener},
    concurrent::executor::SequentialExecutor
};
use pathfinder_gpu::resources::{EmbeddedResourceLoader};
use pathfinder_geometry::{
    vector::{Vector2F, Vector2I},
    rect::RectF
};
use pathfinder_content::color::ColorF;

use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
    dpi::{LogicalSize, LogicalPosition, PhysicalSize, PhysicalPosition},
};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};
use wasm_bindgen::JsCast;
use std::cell::RefCell;

struct Listener<F>(RefCell<F>);
impl<F: FnMut(RenderCommand)> RenderCommandListener for Listener<F> {
    fn send(&self, command: RenderCommand) {
        let mut guard = self.0.borrow_mut();
        let f = &mut *guard;
        f(command)
    }
}
impl<F: FnMut(RenderCommand)> Listener<F> {
    fn new(f: F) -> Self {
        Listener(RefCell::new(f))
    }
}

// we don't have threads on wasm.
#[cfg(target_arch="wasm32")]
unsafe impl<F: FnMut(RenderCommand)> Send for Listener<F> {}
#[cfg(target_arch="wasm32")]
unsafe impl<F: FnMut(RenderCommand)> Sync for Listener<F> {}

pub struct WebGlWindow {
    window: Window,
    renderer: Renderer<WebGlDevice>,
    framebuffer_size: Vector2I
}
impl WebGlWindow {
    pub fn new<T>(event_loop: &EventLoop<T>, canvas_id: &str) -> Self {
        let canvas: HtmlCanvasElement =  web_sys::window().unwrap()
            .document().unwrap()
            .get_element_by_id(canvas_id).unwrap()
            .dyn_into().unwrap();

        let context: WebGl2RenderingContext = canvas
            .get_context("webgl2").unwrap().unwrap()
            .dyn_into().unwrap();
        
        let window = WindowBuilder::new()
            .with_canvas(Some(canvas))
            .build(event_loop)
            .unwrap();
        
        let PhysicalSize { width, height } = window.inner_size();
        let window_size = Vector2F::new(width as _, height as _);

        let dpi = window.scale_factor() as f32;
        let mut framebuffer_size = window_size.scale(dpi).to_i32();
        // Create a Pathfinder renderer.
        let mut renderer = Renderer::new(WebGlDevice::new(context),
            &EmbeddedResourceLoader,
            DestFramebuffer::full_window(framebuffer_size),
            RendererOptions { background_color: Some(ColorF::new(0.9, 0.85, 0.8, 1.0)) }
        );

        WebGlWindow {
            window,
            renderer,
            framebuffer_size,
        }
    }

    pub fn render_as_blob_url(scene: &Scene) -> String {
        use pathfinder_export::{FileFormat, Export};
        use js_sys::{Array, Uint8Array};
        use web_sys::{Blob, console, BlobPropertyBag, Url};

        let mut out = Vec::new();
        scene.export(&mut out, FileFormat::SVG).unwrap();

        let mut bag = BlobPropertyBag::new();
        bag.type_("image/svg+xml");
        let blob = Blob::new_with_u8_array_sequence_and_options(
            &Array::of1(&Uint8Array::from(out.as_slice())),
            &bag
        ).unwrap();
        Url::create_object_url_with_blob(&blob).unwrap()
    }

    pub fn render(&mut self, mut scene: Scene, options: BuildOptions) {
        debug!("render");
        scene.set_view_box(RectF::new(Vector2F::default(), self.framebuffer_size().to_f32()));
        self.renderer.begin_scene();
        scene.build(options, Listener::new(|cmd| {
            debug!("{:?}", cmd);
            self.renderer.render_command(&cmd);
        }), &SequentialExecutor);
        self.renderer.end_scene();
    }
    
    pub fn resize(&mut self, size: Vector2F) {
        let new_framebuffer_size = size.to_i32();
        if new_framebuffer_size != self.framebuffer_size {
            self.framebuffer_size = new_framebuffer_size;
            self.window.set_inner_size(PhysicalSize::new(self.framebuffer_size.x() as u32, self.framebuffer_size.y() as u32));
            self.renderer.replace_dest_framebuffer(DestFramebuffer::full_window(self.framebuffer_size));
        }
    }
    pub fn scale_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
    pub fn framebuffer_size(&self) -> Vector2I {
        self.framebuffer_size
    }
}