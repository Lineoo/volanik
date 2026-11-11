#![windows_subsystem = "windows"]

use std::time::{Duration, Instant};

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
    skybox: Skybox,
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
        let skybox = Skybox::new(options, &camera_layout);
        let mut overlay = Overlay::new(options.device, options.format, pixels_per_point);
        if std::env::var_os("VOLANIK_HIDE_UI").is_some() {
            overlay.set_visible(false);
        }
        let mut control = Control::default();
        if let Ok(time) = std::env::var("VOLANIK_TIME")
            && let Ok(seconds) = time.parse::<f64>()
        {
            control.time = Duration::from_secs_f64(seconds.max(0.0));
        }
        if let Ok(pos) = std::env::var("VOLANIK_CAM_POS")
            && let Some([x, y, z]) = parse_f32s::<3>(&pos)
        {
            control.translation = glam::Vec3::new(x, y, z);
        }
        if let Ok(angles) = std::env::var("VOLANIK_CAM_ANGLES")
            && let Some([yaw, pitch, roll]) = parse_f32s::<3>(&angles)
        {
            control.yaw = yaw.to_radians();
            control.pitch = pitch.to_radians();
            control.roll = roll.to_radians();
        }
        if let Some(coverage) = env_f32("VOLANIK_CLOUD_COVERAGE") {
            control.clouds.coverage = coverage;
        }
        if let Some(density) = env_f32("VOLANIK_CLOUD_DENSITY") {
            control.clouds.density = density;
        }
        if let Some(quality) = env_f32("VOLANIK_CLOUD_QUALITY") {
            control.clouds.quality = quality;
        }
        log::debug!(
            "time={}s forward={:?}",
            control.time.as_secs_f32(),
            control.world().transform_vector3(glam::Vec3::NEG_Z),
        );
        let scene = Scene::new(
            options.device,
            &camera_layout,
            control.data(options.update.aspect_ratio()),
        );
        let input = Input::default();

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

        let options = self.render.options_update();
        self.scene
            .write(options.queue, self.control.data(options.aspect_ratio()));

        self.last = Instant::now();
    }

    fn draw(&mut self) {
        self.render.render(&mut self.overlay, |rpass| {
            self.skybox.draw(rpass, &self.scene);
        });
    }
}

fn env_f32(name: &str) -> Option<f32> {
    std::env::var(name).ok()?.trim().parse().ok()
}

fn parse_f32s<const N: usize>(value: &str) -> Option<[f32; N]> {
    let mut values = [0.0; N];
    let mut count = 0;
    for part in value.split(',') {
        if count == N {
            return None;
        }
        values[count] = part.trim().parse().ok()?;
        count += 1;
    }
    (count == N).then_some(values)
}
