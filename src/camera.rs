use std::{f32::consts::FRAC_PI_2, time::Duration};

use glam::{DVec2, EulerRot, Mat3, Mat4, Quat, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

use crate::window::Input;

pub struct Scene {
    buffer: Buffer,
    group: BindGroup,
    data: SceneData,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct SceneData {
    /// ndc -> camera
    pub view: Mat4,
    /// camera -> world
    pub world: Mat4,
    pub time: f32,
    pub cloud_coverage: f32,
    pub cloud_density: f32,
    pub cloud_wind: f32,
    pub cloud_quality: f32,
    pub cloud_base: f32,
    pub cloud_top: f32,
    pub _pad: [u32; 1],
}

// Keep the Rust struct and the WGSL `Scene` struct in sync.
const _: () = assert!(std::mem::size_of::<SceneData>() == 160);

#[derive(Debug, Clone, Copy)]
pub struct Clouds {
    /// 0 = clear, 1 = overcast
    pub coverage: f32,
    /// density multiplier
    pub density: f32,
    /// horizontal wind speed, m/s
    pub wind: f32,
    /// raymarch quality, 0..1
    pub quality: f32,
    /// cloud base altitude, m
    pub base: f32,
    /// cloud top altitude, m
    pub top: f32,
}

impl Default for Clouds {
    fn default() -> Self {
        Self {
            coverage: 0.45,
            density: 1.0,
            wind: 12.0,
            quality: 0.5,
            base: 2000.0,
            top: 6000.0,
        }
    }
}

pub struct Control {
    pub velocity: Vec3,
    pub translation: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub fov: f32,
    pub speed: f32,
    pub time: Duration,
    pub time_paused: bool,
    pub time_scale: f32,
    pub clouds: Clouds,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            translation: Vec3::ZERO,
            yaw: 0.0,
            pitch: 1.68,
            roll: 0.0,
            fov: 1.8,
            speed: 1000.0,
            time: Duration::ZERO,
            time_paused: false,
            time_scale: 1.0,
            clouds: Clouds::default(),
        }
    }
}

impl Control {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn data(&self, aspect: f32) -> SceneData {
        SceneData {
            view: self.view(aspect),
            world: self.world(),
            time: self.time.as_secs_f32(),
            cloud_coverage: self.clouds.coverage,
            cloud_density: self.clouds.density,
            cloud_wind: self.clouds.wind,
            cloud_quality: self.clouds.quality,
            cloud_base: self.clouds.base,
            cloud_top: self.clouds.top,
            _pad: [0; 1],
        }
    }

    /// ndc -> world
    pub fn view(&self, aspect: f32) -> Mat4 {
        self.projection(aspect).inverse_or_zero()
    }

    /// ndc -> world
    pub fn world(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation(), self.translation)
    }

    pub fn mouse_delta(&mut self, delta: DVec2) {
        self.yaw -= delta.x as f32 * 0.001;
        self.pitch -= delta.y as f32 * 0.001;
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

        self.velocity = Mat3::from_quat(self.rotation_yaw()) * dir * self.speed;
        self.translation += self.velocity * dt.as_secs_f32();

        if input.x {
            self.time = self.time.saturating_add(dt * 100);
        }
        if input.z {
            self.time = self.time.saturating_sub(dt * 100);
        }
        if !self.time_paused {
            let scaled = dt.mul_f32(self.time_scale);
            self.time = self.time.saturating_add(scaled);
        }
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

impl Scene {
    pub fn layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("scene"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    pub fn new(device: &Device, layout: &BindGroupLayout, data: SceneData) -> Self {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("scene"),
            contents: bytemuck::bytes_of(&data),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("scene"),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            buffer,
            group,
            data,
        }
    }

    pub fn write(&mut self, queue: &Queue, data: SceneData) {
        self.data = data;
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&data));
    }

    pub fn group(&self) -> &BindGroup {
        &self.group
    }
}
