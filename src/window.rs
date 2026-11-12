use std::sync::Arc;

use glam::DVec2;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ButtonSource, ElementState, MouseButton, PointerSource, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{Volanik, render::Render};

#[derive(Default)]
pub struct MainEventLoop {
    window: Option<MainWindow>,
}

struct MainWindow {
    inner: Arc<dyn Window>,
    drag: MouseDrag,
    main: Volanik,
}

#[derive(Default)]
struct MouseDrag {
    mouse_start: Option<PhysicalPosition<f64>>,
}

#[derive(Default)]
pub struct Input {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub q: bool,
    pub e: bool,
    pub r: bool,
    pub f: bool,
    pub z: bool,
    pub x: bool,
    pub shift: bool,
}

impl MainEventLoop {
    pub fn run(self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(self).unwrap();
    }
}

impl ApplicationHandler for MainEventLoop {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let attr = WindowAttributes::default()
            .with_transparent(true)
            .with_surface_size(PhysicalSize::new(1600, 900))
            .with_title("Volanik");

        let window = Arc::<dyn Window>::from(event_loop.create_window(attr).unwrap());
        let render = pollster::block_on(Render::new(window.clone(), window.surface_size()));
        let pixels_per_point = window.scale_factor() as f32;
        self.window = Some(MainWindow {
            inner: window,
            drag: MouseDrag::default(),
            main: Volanik::create(render, pixels_per_point),
        });
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &mut self.window else {
            return;
        };

        window.main.overlay.on_window_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                self.window = None;
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                window.main.prepare();
                window.main.draw();
                window.inner.request_redraw();
            }
            WindowEvent::SurfaceResized(size) => {
                window.main.render.resize(size);
            }
            WindowEvent::PointerButton {
                state,
                position,
                button: ButtonSource::Mouse(button),
                ..
            } => {
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        if !window.main.overlay.wants_pointer() {
                            window.drag.mouse_start = Some(position);
                        }
                    } else if state == ElementState::Released {
                        window.drag.mouse_start = None;
                    }
                }
            }
            WindowEvent::PointerMoved {
                position,
                source: PointerSource::Mouse,
                ..
            } => {
                if let Some(start) = &mut window.drag.mouse_start {
                    let delta = DVec2::new(position.x - start.x, position.y - start.y);
                    window.main.control.mouse_delta(delta);
                    *start = position;
                }
            }
            WindowEvent::PointerLeft { .. } => {
                window.drag.mouse_start = None;
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                let pressed = event.state == ElementState::Pressed;
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyW) => window.main.input.w = pressed,
                    PhysicalKey::Code(KeyCode::KeyA) => window.main.input.a = pressed,
                    PhysicalKey::Code(KeyCode::KeyS) => window.main.input.s = pressed,
                    PhysicalKey::Code(KeyCode::KeyD) => window.main.input.d = pressed,
                    PhysicalKey::Code(KeyCode::KeyQ) => window.main.input.q = pressed,
                    PhysicalKey::Code(KeyCode::KeyE) => window.main.input.e = pressed,
                    PhysicalKey::Code(KeyCode::KeyR) => window.main.input.r = pressed,
                    PhysicalKey::Code(KeyCode::KeyF) => window.main.input.f = pressed,
                    PhysicalKey::Code(KeyCode::KeyZ) => window.main.input.z = pressed,
                    PhysicalKey::Code(KeyCode::KeyX) => window.main.input.x = pressed,
                    PhysicalKey::Code(KeyCode::ShiftLeft) => window.main.input.shift = pressed,
                    PhysicalKey::Code(KeyCode::F1) if pressed => {
                        window.main.overlay.toggle_visible();
                    }
                    PhysicalKey::Code(KeyCode::F5) if pressed => {
                        let options = window.main.render.options_creation();
                        window.main.skybox.reload(options);
                    }
                    _ => (),
                }
            }
            _ => {}
        }
    }

    fn destroy_surfaces(&mut self, _event_loop: &dyn ActiveEventLoop) {
        self.window = None;
    }
}
