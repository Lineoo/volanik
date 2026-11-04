use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{Volanik, clouds::Clouds, render::Render};

#[derive(Default)]
pub struct MainEventLoop {
    window: Option<MainWindow>,
}

struct MainWindow {
    inner: Arc<dyn Window>,
    main: Volanik,
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
            .with_title("Volanik");

        let window = Arc::<dyn Window>::from(event_loop.create_window(attr).unwrap());
        let render = pollster::block_on(Render::new(window.clone(), window.surface_size()));
        let clouds = Clouds::new(render.render_option());
        self.window = Some(MainWindow {
            inner: window,
            main: Volanik { render, clouds },
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

        match event {
            WindowEvent::CloseRequested => {
                self.window = None;
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                window.main.render();
                window.inner.request_redraw();
            }
            WindowEvent::SurfaceResized(size) => {
                window.main.render.resize(size);
            }
            _ => {}
        }
    }

    fn destroy_surfaces(&mut self, _event_loop: &dyn ActiveEventLoop) {
        self.window = None;
    }
}
