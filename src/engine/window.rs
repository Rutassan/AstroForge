use winit::{
    event::WindowEvent,
    event_loop::EventLoop,
    window::{CursorGrabMode, Window, WindowBuilder},
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

        // Hide and capture the cursor so the player can look around freely from
        // the start of the game. On some platforms locking might fail, so
        // fall back to confining the cursor to the window.
        let _ = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
        window.set_cursor_visible(false);

        Self { window }
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn present(&mut self) {
        // drawing handled in renderer
    }

    pub fn handle_window_event(
        &mut self,
        event: &WindowEvent,
    ) -> Option<winit::dpi::PhysicalSize<u32>> {
        match event {
            WindowEvent::CloseRequested => Some(winit::dpi::PhysicalSize::new(0, 0)),
            WindowEvent::Resized(size) => Some(*size),
            _ => None,
        }
    }
}
