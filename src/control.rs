use std::{f32::consts::FRAC_PI_2, time::Duration};

use glam::{DVec2, EulerRot, Mat3, Mat4, Quat, Vec3};

use crate::{
    scene::{SceneGroup, SceneUniform},
    skybox::CloudsUniform,
    window::Input,
};

#[derive(Default)]
pub struct Control {
    pub character: Character,
    pub camera: Camera,
    pub cycle: Cycle,
    pub clouds: Clouds,
}

pub struct Character {
    pub speed: f32,
}

pub struct Camera {
    pub translation: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub fov: f32,
}

pub struct Cycle {
    pub time: Duration,
    pub time_paused: bool,
    pub time_scale: f32,
}

pub struct Clouds {
    pub density: f32,
    pub step: i32,
    pub length: f32,
    pub scatter: f32,
    pub absorb: f32,
    pub ior: f32,
}

impl Default for Character {
    fn default() -> Self {
        Self { speed: 1000.0 }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            yaw: 0.0,
            pitch: 1.68,
            roll: 0.0,
            fov: 1.8,
        }
    }
}

impl Default for Cycle {
    fn default() -> Self {
        Self {
            time: Duration::ZERO,
            time_paused: false,
            time_scale: 1.0,
        }
    }
}

impl Default for Clouds {
    fn default() -> Self {
        Self {
            density: 1.0,
            step: 4,
            length: 4e3,
            scatter: 15e-3,
            absorb: 0.0,
            ior: -0.6
        }
    }
}

impl Camera {
    /// ndc -> world
    pub fn view(&self, aspect: f32) -> Mat4 {
        self.projection(aspect).inverse_or_zero()
    }

    /// ndc -> world
    pub fn world(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation(), self.translation)
    }

    fn projection(&self, aspect: f32) -> Mat4 {
        glam::camera::rh::proj::directx::perspective(self.fov, aspect, 0.1, 1000.0)
    }

    fn rotation(&self) -> Quat {
        Quat::from_euler(EulerRot::ZXZ, self.yaw, self.pitch, self.roll)
    }

    fn rotation_yaw(&self) -> Quat {
        Quat::from_euler(EulerRot::ZXZ, self.yaw, FRAC_PI_2, 0.0)
    }
}

impl Control {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn scene(&self, aspect: f32) -> SceneGroup {
        SceneGroup {
            scene: SceneUniform {
                view: self.camera.view(aspect),
                world: self.camera.world(),
                time: self.cycle.time.as_secs_f32(),
                _pad: [0; 3],
            },
            clouds: CloudsUniform {
                density: self.clouds.density,
                step: self.clouds.step,
                dist: self.clouds.length / self.clouds.step as f32,
                scatter: self.clouds.scatter,
                absorb: self.clouds.absorb,
                ior: self.clouds.ior,
                _pad: [0; 2],
            },
        }
    }

    pub fn mouse_delta(&mut self, delta: DVec2) {
        self.camera.yaw -= delta.x as f32 * 0.001;
        self.camera.pitch -= delta.y as f32 * 0.001;
    }

    pub fn input(&mut self, input: &Input, dt: Duration) {
        let mut dir = Vec3::ZERO;
        let scalar = |x: bool| x.then_some(1f32).unwrap_or(0f32);
        dir.z -= scalar(input.w);
        dir.z += scalar(input.s);
        dir.x -= scalar(input.a);
        dir.x += scalar(input.d);
        dir.y -= scalar(input.f);
        dir.y += scalar(input.r);

        let rot = Mat3::from_quat(self.camera.rotation_yaw());
        self.camera.translation += rot * dir * self.character.speed * dt.as_secs_f32();

        if input.x {
            self.cycle.time = self.cycle.time.saturating_add(dt * 100);
        }
        if input.z {
            self.cycle.time = self.cycle.time.saturating_sub(dt * 100);
        }
        if !self.cycle.time_paused {
            let scaled = dt.mul_f32(self.cycle.time_scale);
            self.cycle.time = self.cycle.time.saturating_add(scaled);
        }
    }
}
