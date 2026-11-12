#![windows_subsystem = "windows"]

use std::time::Instant;

use crate::{
    control::Control, overlay::Overlay, render::Render, scene::Scene, skybox::Skybox, window::Input,
};

mod control;
mod overlay;
mod render;
mod scene;
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
    skybox: Skybox,
    scene: Scene,
    control: Control,
    input: Input,
    overlay: Overlay,
    last: Instant,
}

impl Volanik {
    fn create(render: Render, pixels_per_point: f32) -> Self {
        let control = Control::default();
        let input = Input::default();

        let options = render.options_creation();
        let scene_layout = Scene::layout(options.device);
        let skybox = Skybox::new(options, &scene_layout);
        let scene = Scene::new(options.device, &scene_layout);
        let overlay = Overlay::new(options.device, options.format, pixels_per_point);

        Self {
            render,
            skybox,
            scene,
            control,
            input,
            overlay,
            last: Instant::now(),
        }
    }

    fn prepare(&mut self) {
        let dt = self.last.elapsed();

        match self.overlay.wants_keyboard() {
            true => self.control.input(&Input::default(), dt),
            false => self.control.input(&self.input, dt),
        }

        self.overlay
            .frame(&mut self.control, &mut self.skybox, &mut self.render);

        let options = self.render.options_creation();
        self.scene.write(
            options.device,
            options.update.queue,
            &self.control.scene(options.update.aspect_ratio()),
        );

        self.last = Instant::now();
    }

    fn draw(&mut self) {
        self.render.render(&mut self.overlay, |rpass| {
            self.skybox.draw(rpass, &self.scene);
        });
    }
}
