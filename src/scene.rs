use glam::Mat4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};

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
    pub _pad: [u32; 3],
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
