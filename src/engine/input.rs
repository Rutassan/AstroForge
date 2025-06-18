use std::collections::HashSet;
use winit::event::{DeviceEvent, ElementState, Event, VirtualKeyCode, WindowEvent};

#[derive(Default)]
pub struct InputState {
    pressed: HashSet<VirtualKeyCode>,
    pub mouse_delta: (f32, f32),
}

impl InputState {
    pub fn handle_event(&mut self, event: &Event<()>) {
        if let Event::DeviceEvent { event, .. } = event {
            if let DeviceEvent::MouseMotion { delta } = event {
                self.mouse_delta.0 += delta.0 as f32;
                self.mouse_delta.1 += delta.1 as f32;
            }
        }
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::KeyboardInput { input, .. } = event {
                if let Some(key) = input.virtual_keycode {
                    match input.state {
                        ElementState::Pressed => {
                            self.pressed.insert(key);
                        }
                        ElementState::Released => {
                            self.pressed.remove(&key);
                        }
                    }
                }
            }
        }
    }

    pub fn pressed(&self, key: VirtualKeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn reset(&mut self) {
        self.mouse_delta = (0.0, 0.0);
    }
}
