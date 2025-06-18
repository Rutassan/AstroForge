use winit::{
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct WindowState {
    pub window: Window,
}

impl WindowState {
    pub fn new(event_loop: &EventLoop<()>, title: &str, width: u32, height: u32) -> Self {
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(width, height))
            .build(event_loop)
            .expect("failed to create window");
        Self { window }
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn present(&mut self) {
        // TODO: rendering pipeline
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        matches!(event, WindowEvent::CloseRequested)
    }
}
