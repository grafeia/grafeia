
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::{
    concurrent::{
        rayon::RayonExecutor,
        scene_proxy::SceneProxy
    },
    gpu::{
        options::{DestFramebuffer, RendererOptions},
        renderer::Renderer
    },
    scene::Scene,
    options::{BuildOptions, RenderTransform}
};
use pathfinder_gpu::resources::{EmbeddedResourceLoader};
use pathfinder_geometry::{
    vector::{Vector2F, Vector2I},
    rect::RectF
};
use pathfinder_content::color::ColorF;

use glutin::{GlRequest, Api, WindowedContext, PossiblyCurrent};
use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    dpi::{LogicalSize, LogicalPosition, PhysicalSize, PhysicalPosition},
};
use gl;

pub struct GlWindow {
    windowed_context: WindowedContext<PossiblyCurrent>,
    proxy: SceneProxy,
    renderer: Renderer<GLDevice>,
    framebuffer_size: Vector2I
}
impl GlWindow {
    pub fn new<T>(event_loop: &EventLoop<T>, title: String, window_size: Vector2F) -> Self {
        let window_builder = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(LogicalSize::new(window_size.x() as f64, window_size.y() as f64));

        let windowed_context = glutin::ContextBuilder::new()
            .with_gl(GlRequest::Specific(Api::OpenGl, (3, 0)))
            .build_windowed(window_builder, &event_loop)
            .unwrap();
        
        let windowed_context = unsafe {
            windowed_context.make_current().unwrap()
        };

        gl::load_with(|ptr| windowed_context.get_proc_address(ptr));
        
        let dpi = windowed_context.window().scale_factor() as f32;
        let proxy = SceneProxy::new(RayonExecutor);
        let mut framebuffer_size = window_size.scale(dpi).to_i32();
        // Create a Pathfinder renderer.
        let mut renderer = Renderer::new(GLDevice::new(GLVersion::GL3, 0),
            &EmbeddedResourceLoader,
            DestFramebuffer::full_window(framebuffer_size),
            RendererOptions { background_color: Some(ColorF::new(0.9, 0.85, 0.8, 1.0)) }
        );

        GlWindow {
            windowed_context,
            proxy,
            renderer,
            framebuffer_size,
        }
    }
    pub fn render(&mut self, scene: Scene, options: BuildOptions) {
        self.proxy.replace_scene(scene);
        self.proxy.set_view_box(RectF::new(Vector2F::default(), self.framebuffer_size().to_f32()));

        self.proxy.build_and_render(&mut self.renderer, options);
        self.windowed_context.swap_buffers().unwrap();
    }
    
    pub fn resize(&mut self, size: Vector2F) {
        let new_framebuffer_size = size.to_i32();
        if new_framebuffer_size != self.framebuffer_size {
            self.framebuffer_size = new_framebuffer_size;
            self.windowed_context.resize(PhysicalSize::new(self.framebuffer_size.x() as u32, self.framebuffer_size.y() as u32));
            self.renderer.replace_dest_framebuffer(DestFramebuffer::full_window(self.framebuffer_size));
        }
    }
    pub fn scale_factor(&self) -> f32 {
        self.windowed_context.window().scale_factor() as f32
    }
    pub fn request_redraw(&self) {
        self.windowed_context.window().request_redraw();
    }
    pub fn framebuffer_size(&self) -> Vector2I {
        self.framebuffer_size
    }
}