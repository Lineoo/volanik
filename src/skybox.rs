use wgpu::*;

use crate::{render::RenderCreationOptions, scene::Scene, shaders::Shaders};

pub struct Skybox {
    layout: PipelineLayout,
    pipeline: RenderPipeline,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct CloudsUniform {
    pub density: f32,
    pub step: i32,
    pub dist: f32,
    pub scatter: f32,
    pub absorb: f32,
    pub ior: f32,
    pub light_step: i32,
    pub light_dist: f32,
    pub atmos_step: i32,
    pub index: f32,
}

impl Default for CloudsUniform {
    fn default() -> Self {
        Self {
            density: 0.22,
            step: 6,
            dist: 8.6e3,
            scatter: 1e-3,
            absorb: 0.0,
            ior: 0.35,
            light_step: 4,
            light_dist: 2e3,
            atmos_step: 2,
            index: 1.42,
        }
    }
}

impl Skybox {
    pub fn new(options: RenderCreationOptions, camera: &BindGroupLayout) -> Skybox {
        let device = options.device;
        let errors = device.push_error_scope(ErrorFilter::Validation);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("skybox"),
            bind_group_layouts: &[Some(camera)],
            immediate_size: 0,
        });

        let mut shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("skybox"),
            source: ShaderSource::Wgsl(Shaders::Skybox.source()),
        });

        if let Some(errors) = pollster::block_on(errors.pop()) {
            log::warn!("{errors}");
            shader = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("skybox"),
                source: ShaderSource::Wgsl(Shaders::Fallback.source()),
            });
        }

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("skybox"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: options.multisample,
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: options.format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Skybox { layout, pipeline }
    }

    pub fn reload(&mut self, options: RenderCreationOptions) {
        let device = options.device;
        let errors = device.push_error_scope(ErrorFilter::Validation);

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("skybox"),
            source: ShaderSource::Wgsl(Shaders::Skybox.source_fs()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("skybox"),
            layout: Some(&self.layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: options.multisample,
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: options.format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        if let Some(errors) = pollster::block_on(errors.pop()) {
            log::warn!("{errors}");
        } else {
            log::debug!("skybox reload");
            self.pipeline = pipeline;
        }
    }

    pub fn draw(&self, rpass: &mut RenderPass<'_>, camera: &Scene) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &camera.group, &[]);
        rpass.draw(0..3, 0..1);
    }
}
