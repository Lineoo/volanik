#![windows_subsystem = "windows"]

use std::time::Instant;

use glam::Vec3;

use crate::{
    camera::{Camera, Scene, SceneData},
    render::Render,
    skybox::Skybox,
    window::Input,
};

mod camera;
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
    camera: Camera,
    input: Input,
    start: Instant,
    last: Instant,
}

impl Volanik {
    fn create(render: Render) -> Self {
        let options = render.options_creation();
        let camera_layout = Scene::layout(options.device);
        let clouds = Skybox::new(options, &camera_layout);
        let camera = Camera {
            translation: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 1.68,
            roll: 0.0,
            fov: 1.8,
        };
        let scene = Scene::new(
            options.device,
            &camera_layout,
            SceneData {
                view: camera.view(options.update.aspect_ratio()),
                time: 0.0,
                _pad: [0; 3],
            },
        );
        let input = Input::default();

        let now = Instant::now();

        Self {
            render,
            clouds,
            scene,
            camera,
            input,
            start: now,
            last: now,
        }
    }

    fn prepare(&mut self) {
        let options = self.render.options_update();
        let dt = self.last.elapsed();
        self.camera.input(&self.input, dt);
        self.scene.write(
            options.queue,
            SceneData {
                view: self.camera.view(options.aspect_ratio()),
                time: self.start.elapsed().as_secs_f32(),
                _pad: [0; 3],
            },
        );
        self.last = Instant::now();
    }

    fn render(&self) {
        self.render.render(|rpass| {
            self.clouds.render(rpass, &self.scene);
        });
    }
}
