use glam::Mat4;
use wgpu::*;

use crate::skybox::CloudsUniform;

pub struct Scene {
    pub buffer: Buffer,
    pub group: BindGroup,
}

pub struct SceneGroup {
    pub scene: SceneUniform,
    pub clouds: CloudsUniform,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct SceneUniform {
    /// ndc -> camera
    pub view: Mat4,
    /// camera -> world
    pub world: Mat4,
    pub time: f32,
    pub _pad: [u32; 3],
}

impl Scene {
    pub fn layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("scene"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn new(device: &Device, layout: &BindGroupLayout) -> Self {
        let (offsets, size) = Scene::offset_size(device);

        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("scene"),
            size,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("scene"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &buffer,
                        offset: offsets[0],
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &buffer,
                        offset: offsets[1],
                        size: None,
                    }),
                },
            ],
        });

        Self { buffer, group }
    }

    pub fn write(&mut self, device: &Device, queue: &Queue, data: &SceneGroup) {
        let (offsets, _) = Scene::offset_size(device);
        queue.write_buffer(&self.buffer, offsets[0], bytemuck::bytes_of(&data.scene));
        queue.write_buffer(&self.buffer, offsets[1], bytemuck::bytes_of(&data.clouds));
    }

    fn offset_size(device: &Device) -> ([u64; 2], u64) {
        let align = device.limits().min_uniform_buffer_offset_alignment;
        let mut offset = 0;
        (
            std::array::from_fn(|i| {
                let off = offset;
                let size = match i {
                    0 => size_of::<SceneUniform>(),
                    1 => size_of::<CloudsUniform>(),
                    _ => unreachable!(),
                };
                let size = ((((size as u32).saturating_sub(1)) / align) + align) as u64;
                offset += size;
                off
            }),
            offset,
        )
    }
}
