#![windows_subsystem = "windows"]

use std::time::Instant;

use crate::{
    camera::{Control, Scene},
    overlay::Overlay,
    render::Render,
    skybox::Skybox,
    window::Input,
};

mod camera;
mod overlay;
mod render;
mod shaders;
mod skybox;
mod window;

fn main() {
    env_logger::init();

    log::info!("This is Volanik. Welcome!");

    let window = window::MainEventLoop::default();
    window.run();
}

struct Volanik {
    render: Render,
    clouds: Skybox,
    scene: Scene,
    control: Control,
    input: Input,
    overlay: Overlay,
    last: Instant,
}

impl Volanik {
    fn create(render: Render, pixels_per_point: f32) -> Self {
        let options = render.options_creation();
        let camera_layout = Scene::layout(options.device);
        let clouds = Skybox::new(options, &camera_layout);
        let overlay = Overlay::new(options.device, options.format, pixels_per_point);
        let control = Control::default();
        let scene = Scene::new(
            options.device,
            &camera_layout,
            control.data(options.update.aspect_ratio()),
        );
        let input = Input::default();

        Self {
            render,
            clouds,
            scene,
            control,
            input,
            overlay,
            last: Instant::now(),
        }
    }

    fn prepare(&mut self) {
        let options = self.render.options_update();
        let dt = self.last.elapsed();

        match self.overlay.wants_keyboard() {
            true => self.control.input(&Input::default(), dt),
            false => self.control.input(&self.input, dt),
        }

        self.overlay.frame(options.size, &mut self.control);

        self.scene
            .write(options.queue, self.control.data(options.aspect_ratio()));

        self.last = Instant::now();
    }

    fn draw(&mut self) {
        self.render.render(&mut self.overlay, |rpass| {
            self.clouds.draw(rpass, &self.scene);
        });
    }
}
