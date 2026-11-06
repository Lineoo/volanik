use wgpu::*;
use winit::dpi::PhysicalSize;

const MSAA_SAMPLE_COUNT: u32 = 1;

pub struct Render {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    resolve: Option<Texture>,

    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,

    clear_color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderCreationOptions<'r> {
    pub device: &'r Device,
    pub multisample: MultisampleState,
    pub format: TextureFormat,
    pub update: RenderUpdateOptions<'r>,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderUpdateOptions<'r> {
    pub queue: &'r Queue,
    pub size: PhysicalSize<u32>,
}

impl RenderUpdateOptions<'_> {
    pub fn aspect_ratio(&self) -> f32 {
        self.size.width as f32 / self.size.height as f32
    }
}

impl Render {
    pub async fn new(window: impl Into<SurfaceTarget<'static>>, size: PhysicalSize<u32>) -> Render {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            flags: InstanceFlags::default(),
            memory_budget_thresholds: MemoryBudgetThresholds::default(),
            backend_options: BackendOptions::default(),
            display: None,
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .unwrap();

        log::debug!("wgpu adapter: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: None,
                required_features: Features::default(),
                required_limits: Limits::defaults(),
                experimental_features: ExperimentalFeatures::default(),
                memory_hints: MemoryHints::MemoryUsage,
                trace: Trace::Off,
            })
            .await
            .unwrap();

        let config = configuration(&surface, &adapter, size);
        surface.configure(&device, &config);

        let resolve =
            (MSAA_SAMPLE_COUNT > 1).then(|| device.create_texture(&msaa_texture(&config)));

        Render {
            surface,
            config,
            resolve,
            instance,
            adapter,
            device,
            queue,
            clear_color: Color::WHITE,
        }
    }

    #[expect(unused)]
    pub fn recreate(&mut self, window: impl Into<SurfaceTarget<'static>>, size: PhysicalSize<u32>) {
        self.surface = self.instance.create_surface(window).unwrap();
        self.config = configuration(&self.surface, &self.adapter, size);
        self.surface.configure(&self.device, &self.config);
        self.resolve = (MSAA_SAMPLE_COUNT > 1)
            .then(|| self.device.create_texture(&msaa_texture(&self.config)));
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config = configuration(&self.surface, &self.adapter, size);
        self.surface.configure(&self.device, &self.config);
        self.resolve = (MSAA_SAMPLE_COUNT > 1)
            .then(|| self.device.create_texture(&msaa_texture(&self.config)));
    }

    pub fn options_creation(&self) -> RenderCreationOptions<'_> {
        RenderCreationOptions {
            device: &self.device,
            multisample: {
                MultisampleState {
                    count: MSAA_SAMPLE_COUNT,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                }
            },
            format: self.config.format,
            update: self.options_update(),
        }
    }

    pub fn options_update(&self) -> RenderUpdateOptions<'_> {
        RenderUpdateOptions {
            queue: &self.queue,
            size: PhysicalSize::new(self.config.width, self.config.height),
        }
    }

    pub fn render(&self, f: impl FnOnce(&mut RenderPass)) {
        let surface = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            CurrentSurfaceTexture::Timeout => return,
            CurrentSurfaceTexture::Occluded => return,
            CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            CurrentSurfaceTexture::Lost => return,
            CurrentSurfaceTexture::Validation => return,
        };

        let msaa_view = self
            .resolve
            .as_ref()
            .map(|x| x.create_view(&TextureViewDescriptor::default()));
        let surface_view = surface
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("main_encoder"),
            });

        let mut rpass = if let Some(msaa_view) = msaa_view {
            msaa_rpass(self.clear_color, &msaa_view, &surface_view, &mut encoder)
        } else {
            plain_rpass(self.clear_color, &surface_view, &mut encoder)
        };

        f(&mut rpass);

        drop(rpass);
        self.queue.submit([encoder.finish()]);
        self.queue.present(surface);
    }
}

fn configuration(
    surface: &Surface,
    adapter: &Adapter,
    size: PhysicalSize<u32>,
) -> SurfaceConfiguration {
    let caps = surface.get_capabilities(&adapter);
    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: (caps.formats.iter().copied())
            .max_by_key(|format| match format {
                TextureFormat::Rgba8UnormSrgb => 100,
                TextureFormat::Bgra8UnormSrgb => 90,
                _ if format.is_srgb() => 10,
                _ => 0,
            })
            .unwrap(),
        width: size.width.max(1),
        height: size.height.max(1),
        desired_maximum_frame_latency: 2,
        present_mode: (caps.present_modes.iter().copied())
            .max_by_key(|mode| match mode {
                PresentMode::AutoVsync => 10,
                PresentMode::Fifo => 10,
                PresentMode::FifoRelaxed => 10,
                _ => 0,
            })
            .unwrap(),
        alpha_mode: (caps.alpha_modes.iter().copied())
            .max_by_key(|mode| match mode {
                CompositeAlphaMode::Opaque => 10,
                _ => 0,
            })
            .unwrap(),
        color_space: SurfaceColorSpace::Auto,
        view_formats: vec![],
    };

    log::debug!("surface size {}x{}", config.width, config.height);
    log::debug!("texture format {:?}", config.format);
    log::debug!("present mode {:?}", config.present_mode);
    log::debug!("alpha mode {:?}", config.alpha_mode);

    config
}

fn msaa_rpass<'encoder>(
    clear_color: Color,
    msaa_view: &TextureView,
    surface_view: &TextureView,
    encoder: &'encoder mut CommandEncoder,
) -> RenderPass<'encoder> {
    encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("main_rpass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &msaa_view,
            resolve_target: Some(surface_view),
            ops: Operations {
                load: LoadOp::Clear(clear_color),
                store: StoreOp::Discard,
            },
            depth_slice: None,
        })],
        timestamp_writes: None,
        ..Default::default()
    })
}

fn plain_rpass<'encoder>(
    clear_color: Color,
    surface_view: &TextureView,
    encoder: &'encoder mut CommandEncoder,
) -> RenderPass<'encoder> {
    encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("main_rpass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: surface_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(clear_color),
                store: StoreOp::Store,
            },
            depth_slice: None,
        })],
        timestamp_writes: None,
        ..Default::default()
    })
}

fn msaa_texture(config: &SurfaceConfiguration) -> TextureDescriptor<'static> {
    assert!(
        MSAA_SAMPLE_COUNT > 1,
        "msaa texture should be created only when msaa sample count > 1"
    );
    TextureDescriptor {
        label: Some("render_msaa"),
        size: Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: MSAA_SAMPLE_COUNT,
        dimension: TextureDimension::D2,
        format: config.format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TRANSIENT_ATTACHMENT,
        view_formats: &[],
    }
}
