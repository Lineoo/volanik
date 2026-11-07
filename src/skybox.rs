use wgpu::*;

use crate::{camera::Scene, render::RenderCreationOptions, shaders::Shaders};

pub struct Skybox {
    pipeline: RenderPipeline,
}

impl Skybox {
    pub fn new(options: RenderCreationOptions, camera: &BindGroupLayout) -> Skybox {
        let device = options.device;

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("skybox"),
            bind_group_layouts: &[Some(camera)],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("skybox"),
            source: ShaderSource::Wgsl(Shaders::Skybox.source()),
        });

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

        Skybox { pipeline }
    }

    pub fn draw(&self, rpass: &mut RenderPass<'_>, camera: &Scene) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, camera.group(), &[]);
        rpass.draw(0..3, 0..1);
    }
}
