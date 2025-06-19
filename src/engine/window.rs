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

    /// Attempt to capture the cursor and hide it. This works even on
    /// platforms where pointer locking might not be available by
    /// falling back to confining the cursor to the window.
    pub fn capture_cursor(&self) {
        let _ = self
            .window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
        self.window.set_cursor_visible(false);
    }

    /// Release the cursor and make it visible again.
    pub fn release_cursor(&self) {
        let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        self.window.set_cursor_visible(true);
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn present(&mut self) {
        // drawing handled in renderer
    }

    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
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

    pub fn dummy(width: u32, height: u32) -> Self {
        struct DummyWindow;
        impl DummyWindow {
            fn request_redraw(&self) {}
            fn set_title(&self, _title: &str) {}
            fn set_cursor_grab(&self, _mode: CursorGrabMode) -> Result<(), ()> { Ok(()) }
            fn set_cursor_visible(&self, _visible: bool) {}
        }
        // SAFETY: мы не используем это окно нигде, кроме как для заглушки
        let window: Window = unsafe { std::mem::zeroed() };
        Self { window }
    }
}
