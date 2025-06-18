pub mod audio;
pub mod input;
pub mod physics;
pub mod renderer;
pub mod window;

use audio::AudioSystem;
use input::InputState;
use renderer::Renderer;
use window::WindowState;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

pub struct Engine {
    pub event_loop: Option<EventLoop<()>>,
    pub window: WindowState,
    pub input: InputState,
    pub audio: AudioSystem,
    pub renderer: Renderer,
}

impl Engine {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        let event_loop = EventLoop::new();
        let window = WindowState::new(&event_loop, title, width, height);
        let renderer = futures_lite::future::block_on(Renderer::new(&window.window));
        Self {
            event_loop: Some(event_loop),
            window,
            input: InputState::default(),
            audio: AudioSystem::new(),
            renderer,
        }
    }

    pub fn run<F: FnMut(&mut Self) + 'static>(mut self, mut update: F) {
        let event_loop = self.event_loop.take().unwrap();
        let mut engine = self;
        event_loop.run(move |event, _, control_flow| {
            engine.input.handle_event(&event);
            match event {
                Event::MainEventsCleared => {
                    update(&mut engine);
                    engine.window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    engine.renderer.render();
                }
                Event::WindowEvent { ref event, .. } => {
                    if let Some(size) = engine.window.handle_window_event(event) {
                        if size.width == 0 && size.height == 0 {
                            *control_flow = ControlFlow::Exit;
                        } else {
                            engine.renderer.resize(size);
                        }
                    }
                }
                _ => {}
            }
        });
    }
}
