use wgpu::*;

use crate::{render::RenderPipelineOptions, shaders::Shaders};

pub struct Clouds {
    pipeline: RenderPipeline,
}

impl Clouds {
    pub fn new(options: RenderPipelineOptions) -> Clouds {
        let device = options.device;

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("clouds"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("clouds"),
            source: ShaderSource::Wgsl(Shaders::Clouds.source()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("clouds"),
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

        Clouds { pipeline }
    }

    pub fn render(&self, rpass: &mut RenderPass<'_>) {
        rpass.set_pipeline(&self.pipeline);
        rpass.draw(0..3, 0..1);
    }
}
