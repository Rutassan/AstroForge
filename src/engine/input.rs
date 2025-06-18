use std::collections::HashSet;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};

#[derive(Default)]
pub struct InputState {
    pressed: HashSet<VirtualKeyCode>,
}

impl InputState {
    pub fn handle_event(&mut self, event: &Event<()>) {
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
}
