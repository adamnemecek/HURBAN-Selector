use std::cmp::Ordering;

use wgpu::winit;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InputState {
    pub camera_pan_ground: [f32; 2],
    pub camera_pan_screen: [f32; 2],
    pub camera_rotate: [f32; 2],
    pub camera_zoom: f32,
    pub camera_zoom_steps: i32,
    pub camera_reset_viewport: bool,
    pub close_requested: bool,
    pub window_resized: Option<winit::dpi::LogicalSize>,
}

#[derive(Debug)]
pub struct InputManager {
    lmb_down: bool,
    rmb_down: bool,
    shift_down: bool,
    input_state: InputState,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            lmb_down: false,
            rmb_down: false,
            shift_down: false,
            input_state: InputState::default(),
        }
    }

    pub fn input_state(&self) -> &InputState {
        &self.input_state
    }

    pub fn start_frame(&mut self) {
        self.input_state = InputState::default();
    }

    pub fn process_event(&mut self, ev: winit::Event) {
        // FIXME: these should come in as parameters
        let gui_captured_keyboard: bool = false;
        let gui_captured_mouse: bool = false;

        match ev {
            winit::Event::WindowEvent { event, .. } => match event {
                winit::WindowEvent::CloseRequested => {
                    self.input_state.close_requested = true;
                }
                winit::WindowEvent::KeyboardInput { input, .. } => {
                    let winit::KeyboardInput {
                        virtual_keycode,
                        state,
                        modifiers,
                        ..
                    } = input;

                    // We respond to some events unconditionally, even if GUI has focus.
                    match (virtual_keycode, state, modifiers) {
                        // Cmd+Q for macOS
                        #[cfg(target_os = "macos")]
                        (
                            Some(winit::VirtualKeyCode::Q),
                            winit::ElementState::Pressed,
                            winit::ModifiersState {
                                logo: true,
                                shift: false,
                                ctrl: false,
                                alt: false,
                            },
                        ) => {
                            self.input_state.close_requested = true;
                        }
                        (Some(winit::VirtualKeyCode::LShift), winit::ElementState::Pressed, _) => {
                            self.shift_down = true;
                        }
                        (Some(winit::VirtualKeyCode::LShift), winit::ElementState::Released, _) => {
                            self.shift_down = false;
                        }
                        (Some(winit::VirtualKeyCode::RShift), winit::ElementState::Pressed, _) => {
                            self.shift_down = true;
                        }
                        (Some(winit::VirtualKeyCode::RShift), winit::ElementState::Released, _) => {
                            self.shift_down = false;
                        }
                        _ => (),
                    };

                    // These events are responded to only when gui doesn't have focus
                    if !gui_captured_keyboard {
                        if let (Some(winit::VirtualKeyCode::A), winit::ElementState::Pressed, _) =
                            (virtual_keycode, state, modifiers)
                        {
                            self.input_state.camera_reset_viewport = true;
                        }
                    }
                }
                winit::WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                    (winit::ElementState::Pressed, winit::MouseButton::Left) => {
                        self.lmb_down = true;
                    }
                    (winit::ElementState::Released, winit::MouseButton::Left) => {
                        self.lmb_down = false;
                    }
                    (winit::ElementState::Pressed, winit::MouseButton::Right) => {
                        self.rmb_down = true;
                    }
                    (winit::ElementState::Released, winit::MouseButton::Right) => {
                        self.rmb_down = false;
                    }
                    (_, _) => (),
                },
                winit::WindowEvent::Resized(logical_size) => {
                    // Even if the window resized multiple times, only
                    // take the last one into account.
                    self.input_state.window_resized = Some(logical_size);
                }
                _ => (),
            },
            winit::Event::DeviceEvent { event, .. } => match event {
                winit::DeviceEvent::MouseMotion { delta } => {
                    if !gui_captured_mouse {
                        let x = delta.0 as f32;
                        let y = delta.1 as f32;
                        if self.lmb_down && self.rmb_down {
                            self.input_state.camera_zoom -= y;
                        } else if self.lmb_down {
                            self.input_state.camera_rotate[0] -= x;
                            self.input_state.camera_rotate[1] -= y;
                        } else if self.rmb_down {
                            if self.shift_down {
                                self.input_state.camera_pan_ground[0] += x;
                                self.input_state.camera_pan_ground[1] -= y;
                            } else {
                                self.input_state.camera_pan_screen[0] += x;
                                self.input_state.camera_pan_screen[1] -= y;
                            }
                        }
                    }
                }
                winit::DeviceEvent::MouseWheel { delta, .. } => match delta {
                    winit::MouseScrollDelta::PixelDelta(winit::dpi::LogicalPosition {
                        y, ..
                    }) => {
                        if !gui_captured_mouse {
                            match y.partial_cmp(&0.0) {
                                Some(Ordering::Greater) => self.input_state.camera_zoom_steps += 1,
                                Some(Ordering::Less) => self.input_state.camera_zoom_steps -= 1,
                                _ => (),
                            }
                        }
                    }
                    winit::MouseScrollDelta::LineDelta(_, y) => {
                        if !gui_captured_mouse {
                            match y.partial_cmp(&0.0) {
                                Some(Ordering::Greater) => self.input_state.camera_zoom_steps += 1,
                                Some(Ordering::Less) => self.input_state.camera_zoom_steps -= 1,
                                _ => (),
                            }
                        }
                    }
                },
                _ => (),
            },
            _ => (),
        }
    }
}
