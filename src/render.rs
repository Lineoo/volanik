use std::{cell::Cell, path::PathBuf};

use wgpu::*;
use winit::dpi::PhysicalSize;

use crate::overlay::Overlay;

const MSAA_SAMPLE_COUNT: u32 = 1;
const EXPORT_SETTLE_FRAMES: u32 = 3;

pub struct Render {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    resolve: Option<Texture>,
    export: Option<Export>,

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

struct Export {
    path: PathBuf,
    buffer: Buffer,
    exported: Cell<bool>,
    frames: Cell<u32>,
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

        let export = std::env::var_os("VOLANIK_EXPORT_PATH").map(|p| Export {
            path: PathBuf::from(p),
            buffer: device.create_buffer(&export_buffer(&config)),
            exported: Cell::new(false),
            frames: Cell::new(0),
        });

        Render {
            surface,
            config,
            resolve,
            export,
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
        if let Some(resolve) = &mut self.resolve {
            *resolve = self.device.create_texture(&msaa_texture(&self.config));
        }
        if let Some(export) = &mut self.export {
            export.buffer = self.device.create_buffer(&export_buffer(&self.config));
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config = configuration(&self.surface, &self.adapter, size);
        self.surface.configure(&self.device, &self.config);
        if let Some(resolve) = &mut self.resolve {
            *resolve = self.device.create_texture(&msaa_texture(&self.config));
        }
        if let Some(export) = &mut self.export {
            export.buffer = self.device.create_buffer(&export_buffer(&self.config));
        }
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

    pub fn render(&self, overlay: &mut Overlay, f: impl FnOnce(&mut RenderPass<'static>)) {
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

        if overlay.has_content() {
            overlay.prepare(&self.device, &self.queue, &mut encoder);
        }

        let rpass = if let Some(msaa_view) = msaa_view {
            msaa_rpass(self.clear_color, &msaa_view, &surface_view, &mut encoder)
        } else {
            plain_rpass(self.clear_color, &surface_view, &mut encoder)
        };

        let mut rpass = rpass.forget_lifetime();

        f(&mut rpass);

        if overlay.has_content() {
            overlay.draw(&mut rpass);
            overlay.finish();
        }

        drop(rpass);

        let export_now = self
            .export
            .as_ref()
            .is_some_and(|export| export.should_export());
        if let Some(export) = &self.export
            && export_now
        {
            export.export(&self.config, &surface.texture, &mut encoder);
            self.queue.submit([encoder.finish()]);
            self.queue.present(surface);
            export.save(&self.config, &self.device);
        } else {
            self.queue.submit([encoder.finish()]);
            self.queue.present(surface);
        }
    }
}

impl Export {
    fn should_export(&self) -> bool {
        if self.exported.get() {
            return false;
        }
        let frames = self.frames.get() + 1;
        self.frames.set(frames);
        frames >= EXPORT_SETTLE_FRAMES
    }

    fn export(
        &self,
        config: &SurfaceConfiguration,
        surface: &Texture,
        encoder: &mut CommandEncoder,
    ) {
        if self.exported.get() {
            return;
        }

        let (bytes_per_row, rows_per_image) = export_extend(config);
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: surface,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(rows_per_image),
                },
            },
            Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn save(&self, config: &SurfaceConfiguration, device: &Device) {
        if self.exported.get() {
            return;
        }

        let (row, col) = export_extend(config);
        let (row_png, col_png) = (config.width * 4, config.height);
        let (row, _col) = (row as usize, col as usize);
        let (row_png, col_png) = (row_png as usize, col_png as usize);

        let slice = self.buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(MapMode::Read, move |result| tx.send(result).unwrap());
        device.poll(PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();

        let data = slice.get_mapped_range().unwrap();
        let mut pixels = Vec::with_capacity(row_png * col_png);
        for line in data.chunks(row) {
            pixels.extend_from_slice(&line[..row_png]);
        }
        drop(data);
        self.buffer.unmap();

        // The surface is usually BGRA, but the PNG writer expects RGBA.
        if matches!(
            config.format,
            TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb
        ) {
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }

        if let Some(parent) = self.path.parent().filter(|parent| !parent.is_empty()) {
            std::fs::create_dir_all(parent).unwrap();
        }

        let file = std::fs::File::create(&self.path).unwrap();
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(file), config.width, config.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&pixels).unwrap();
        writer.finish().unwrap();

        log::info!("exported frame to {}", self.path.display());
        self.exported.set(true);

        if std::env::var_os("VOLANIK_EXPORT_EXIT").is_some() {
            std::process::exit(0);
        }
    }
}

fn configuration(
    surface: &Surface,
    adapter: &Adapter,
    size: PhysicalSize<u32>,
) -> SurfaceConfiguration {
    let caps = surface.get_capabilities(&adapter);
    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
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

fn export_buffer(config: &SurfaceConfiguration) -> BufferDescriptor<'static> {
    let (bytes_per_row, rows_per_image) = export_extend(config);
    BufferDescriptor {
        label: Some("export_readback"),
        size: (bytes_per_row as u64) * (rows_per_image as u64),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    }
}

fn export_extend(config: &SurfaceConfiguration) -> (u32, u32) {
    let bytes_per_row =
        (config.width * 4).div_ceil(COPY_BYTES_PER_ROW_ALIGNMENT) * COPY_BYTES_PER_ROW_ALIGNMENT;
    let rows_per_image = config.height;
    (bytes_per_row, rows_per_image)
}
