//! egui debug overlay: input bridge and GPU pass for the winit 0.31 app.

use std::{
    collections::HashSet,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use egui::{
    Color32, Context, DragValue, Event, FontData, FontDefinitions, FontFamily, Key, Modifiers,
    MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, RichText, Slider,
    TouchPhase, Ui, Vec2, ViewportId,
};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use wgpu::{CommandEncoder, Device, Queue, RenderPass, TextureFormat};
use winit::{
    dpi::PhysicalPosition,
    event::{
        ElementState, KeyEvent, MouseButton, MouseScrollDelta, ScrollSource,
        TouchPhase as WinitTouchPhase, WindowEvent,
    },
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    camera::{Clouds, Control},
    render::Render,
    skybox::Skybox,
};

const DAY_SECONDS: f64 = 24.0 * 60.0 * 60.0;
const TARGET_FRAME_SECONDS: f32 = 1.0 / 60.0;

const CJK_FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/adobe-source-han-sans/SourceHanSansCN-Regular.otf",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "C:\\Windows\\Fonts\\msyh.ttc",
    "/System/Library/Fonts/PingFang.ttc",
];

#[derive(Clone, Copy)]
struct FrameStats {
    fps: f32,
    frame_ms: f32,
}

pub struct Overlay {
    context: Context,
    renderer: Renderer,
    raw_input: RawInput,
    paint_jobs: Vec<egui::ClippedPrimitive>,
    textures_delta: egui::TexturesDelta,
    free_textures: Vec<egui::TextureId>,

    screen: ScreenDescriptor,
    native_pixels_per_point: f32,
    pixels_per_point: f32,
    focused: bool,
    modifiers: Modifiers,
    pressed_keys: HashSet<KeyCode>,

    visible: bool,
    started: Instant,
    last_frame: Instant,
    stats: FrameStats,
}

impl Overlay {
    pub fn new(device: &Device, format: TextureFormat, pixels_per_point: f32) -> Self {
        let context = Context::default();
        install_fonts(&context);
        context.set_visuals(egui::Visuals::dark());

        let renderer = Renderer::new(device, format, RendererOptions::default());
        let raw_input = fresh_raw_input(pixels_per_point);

        Self {
            context,
            renderer,
            raw_input,
            paint_jobs: Vec::new(),
            textures_delta: egui::TexturesDelta::default(),
            free_textures: Vec::new(),
            screen: ScreenDescriptor {
                size_in_pixels: [1, 1],
                pixels_per_point,
            },
            native_pixels_per_point: pixels_per_point,
            pixels_per_point,
            focused: true,
            modifiers: Modifiers::NONE,
            pressed_keys: HashSet::new(),
            visible: true,
            started: Instant::now(),
            last_frame: Instant::now(),
            stats: FrameStats {
                fps: 0.0,
                frame_ms: 0.0,
            },
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        if self.visible == visible {
            return;
        }
        self.visible = visible;
        self.raw_input.events.clear();
        self.pressed_keys.clear();
        if !visible {
            self.paint_jobs.clear();
        }
    }

    pub fn toggle_visible(&mut self) {
        self.set_visible(!self.visible);
    }

    pub fn wants_pointer(&self) -> bool {
        self.visible && self.context.egui_wants_pointer_input()
    }

    pub fn wants_keyboard(&self) -> bool {
        self.visible && self.context.egui_wants_keyboard_input()
    }

    pub fn set_pixels_per_point(&mut self, pixels_per_point: f32) {
        if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
            self.native_pixels_per_point = pixels_per_point;
            self.pixels_per_point = pixels_per_point * self.context.zoom_factor();
        }
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) {
        if let WindowEvent::ScaleFactorChanged { scale_factor, .. } = event {
            self.set_pixels_per_point(*scale_factor as f32);
        }
        if !self.visible {
            return;
        }

        match event {
            WindowEvent::Focused(focused) => {
                self.focused = *focused;
                if !focused {
                    self.pressed_keys.clear();
                }
                self.raw_input.focused = *focused;
                self.raw_input.events.push(Event::WindowFocused(*focused));
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = egui_modifiers(modifiers);
                self.raw_input
                    .events
                    .push(Event::ModifiersChanged(self.modifiers));
            }
            WindowEvent::PointerEntered { position, .. }
            | WindowEvent::PointerMoved { position, .. } => {
                self.raw_input
                    .events
                    .push(Event::PointerMoved(self.to_points(*position)));
            }
            WindowEvent::PointerLeft { .. } => {
                self.raw_input.events.push(Event::PointerGone);
            }
            WindowEvent::PointerButton {
                state,
                position,
                button,
                ..
            } => {
                if let Some(button) = button.clone().mouse_button().and_then(pointer_button) {
                    self.raw_input.events.push(Event::PointerButton {
                        pos: self.to_points(*position),
                        button,
                        pressed: *state == ElementState::Pressed,
                        modifiers: self.modifiers,
                    });
                }
            }
            WindowEvent::MouseWheel {
                delta,
                phase,
                source,
                ..
            } => {
                let source = match source {
                    ScrollSource::Wheel => MouseWheelSource::Wheel,
                    ScrollSource::Finger => MouseWheelSource::Trackpad,
                    ScrollSource::Momentum => MouseWheelSource::Momentum,
                    _ => MouseWheelSource::Unknown,
                };
                let phase = egui_touch_phase(*phase);
                match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        self.raw_input.events.push(Event::MouseWheel {
                            unit: MouseWheelUnit::Line,
                            delta: Vec2::new(*x, *y),
                            phase,
                            source,
                            modifiers: self.modifiers,
                        });
                    }
                    MouseScrollDelta::PixelDelta(position) => {
                        self.raw_input.events.push(Event::MouseWheel {
                            unit: MouseWheelUnit::Point,
                            delta: Vec2::new(
                                position.x as f32 / self.pixels_per_point,
                                position.y as f32 / self.pixels_per_point,
                            ),
                            phase,
                            source,
                            modifiers: self.modifiers,
                        });
                    }
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => self.on_key_event(event),
            _ => {}
        }
    }

    fn on_key_event(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        let mut repeat = false;
        let physical_key = match event.physical_key {
            PhysicalKey::Code(code) => {
                if pressed {
                    repeat = !self.pressed_keys.insert(code);
                } else {
                    self.pressed_keys.remove(&code);
                }
                Some(code)
            }
            _ => None,
        };

        if let Some(key) = physical_key.and_then(egui_key) {
            self.raw_input.events.push(Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat,
                modifiers: self.modifiers,
            });
        }

        if pressed
            && !self.modifiers.ctrl
            && !self.modifiers.mac_cmd
            && let Some(text) = &event.text
            && !text.is_empty()
            && !text.chars().any(|c| c.is_control())
        {
            self.raw_input.events.push(Event::Text(text.to_string()));
        }
    }

    pub fn has_content(&self) -> bool {
        self.visible
            && (!self.paint_jobs.is_empty()
                || !self.textures_delta.set.is_empty()
                || !self.free_textures.is_empty())
    }

    pub fn prepare(&mut self, device: &Device, queue: &Queue, encoder: &mut CommandEncoder) {
        for (id, deltas) in std::mem::take(&mut self.textures_delta.set) {
            for delta in deltas {
                self.renderer.update_texture(device, queue, id, &delta);
            }
        }
        self.free_textures
            .extend(std::mem::take(&mut self.textures_delta.free));

        let _ =
            self.renderer
                .update_buffers(device, queue, encoder, &self.paint_jobs, &self.screen);
    }

    pub fn draw(&self, rpass: &mut RenderPass<'static>) {
        self.renderer.render(rpass, &self.paint_jobs, &self.screen);
    }

    pub fn finish(&mut self) {
        for texture in self.free_textures.drain(..) {
            self.renderer.free_texture(&texture);
        }
    }

    pub fn frame(&mut self, control: &mut Control, skybox: &mut Skybox, render: &mut Render) {
        self.tick();

        if !self.visible {
            return;
        }

        let options = render.options_update();
        let size = [options.size.width.max(1), options.size.height.max(1)];
        let native_ppp = self.native_pixels_per_point;
        let ppp = self.pixels_per_point;
        self.screen = ScreenDescriptor {
            size_in_pixels: size,
            pixels_per_point: ppp,
        };

        self.raw_input.focused = self.focused;
        self.raw_input.screen_rect = Some(Rect::from_min_size(
            Pos2::ZERO,
            Vec2::new(size[0] as f32 / ppp, size[1] as f32 / ppp),
        ));
        self.raw_input.time = Some(self.started.elapsed().as_secs_f64());
        self.raw_input.predicted_dt = TARGET_FRAME_SECONDS;
        if let Some(viewport) = self.raw_input.viewports.get_mut(&ViewportId::ROOT) {
            viewport.native_pixels_per_point = Some(native_ppp);
            viewport.focused = Some(self.focused);
        }

        let input = std::mem::take(&mut self.raw_input);
        let mut open = self.visible;
        let panel_info = PanelInfo {
            stats: self.stats,
            screen: size,
            pixels_per_point: ppp,
        };

        let output = self.context.run_ui(input, |ui| {
            egui::Window::new("Volanik")
                .open(&mut open)
                .default_pos([12.0, 12.0])
                .default_size([340.0, 640.0])
                .resizable(true)
                .vscroll(true)
                .show(ui.ctx(), |ui| {
                    overlay_ui(ui, control, skybox, render, panel_info);
                });
        });

        self.visible = open;

        let egui::FullOutput {
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = output;

        self.pixels_per_point = pixels_per_point.max(0.1);
        self.screen.pixels_per_point = self.pixels_per_point;
        self.paint_jobs = self.context.tessellate(shapes, pixels_per_point);
        self.textures_delta = textures_delta;
        self.raw_input = fresh_raw_input(native_ppp);
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        if dt > 0.0 && dt < 1.0 {
            let frame_ms = dt * 1000.0;
            self.stats.frame_ms = if self.stats.frame_ms == 0.0 {
                frame_ms
            } else {
                self.stats.frame_ms * 0.9 + frame_ms * 0.1
            };
            self.stats.fps = 1000.0 / self.stats.frame_ms;
        }
    }

    fn to_points(&self, position: PhysicalPosition<f64>) -> Pos2 {
        Pos2::new(
            position.x as f32 / self.pixels_per_point,
            position.y as f32 / self.pixels_per_point,
        )
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        // egui's texture deltas must be applied, or explicitly dropped.
        self.textures_delta.clear();
    }
}

#[derive(Clone, Copy)]
struct PanelInfo {
    stats: FrameStats,
    screen: [u32; 2],
    pixels_per_point: f32,
}

fn overlay_ui(
    ui: &mut Ui,
    control: &mut Control,
    skybox: &mut Skybox,
    render: &mut Render,
    info: PanelInfo,
) {
    let stats = info.stats;
    ui.horizontal(|ui| {
        let fps_color = if stats.fps >= 55.0 {
            Color32::from_rgb(120, 220, 150)
        } else if stats.fps >= 30.0 {
            Color32::from_rgb(235, 200, 110)
        } else {
            Color32::from_rgb(235, 120, 120)
        };
        ui.label(
            RichText::new(format!("{:>4.0} FPS", stats.fps))
                .monospace()
                .color(fps_color),
        );
        ui.separator();
        ui.label(RichText::new(format!("{:.2} ms", stats.frame_ms)).monospace());
    });
    ui.label(
        RichText::new(format!(
            "{} × {} · {:.2} px/pt",
            info.screen[0], info.screen[1], info.pixels_per_point
        ))
        .small()
        .weak(),
    );

    ui.separator();
    egui::CollapsingHeader::new("相机")
        .default_open(true)
        .show(ui, |ui| camera_ui(ui, control));

    ui.separator();
    egui::CollapsingHeader::new("时间")
        .default_open(true)
        .show(ui, |ui| {
            time_ui(ui, control);
        });

    ui.separator();
    egui::CollapsingHeader::new("云")
        .default_open(true)
        .show(ui, |ui| clouds_ui(ui, &mut control.clouds));

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("重置相机").clicked() {
            control.reset();
        }
        if ui.button("隐藏 (F1)").clicked() {
            ui.close_kind(egui::UiKind::Window);
        }
        if ui.button("重载").clicked() {
            let options = render.options_creation();
            skybox.reload(options);
        }
    });
    ui.small("F1 显示/隐藏面板 · 拖拽鼠标旋转视角");
}

fn camera_ui(ui: &mut Ui, camera: &mut Control) {
    egui::Grid::new("camera_position")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("位置");
            ui.horizontal(|ui| {
                ui.add(
                    DragValue::new(&mut camera.translation.x)
                        .speed(1.0)
                        .prefix("X ")
                        .suffix(" m"),
                )
                .on_hover_text("世界 X 坐标");
                ui.add(
                    DragValue::new(&mut camera.translation.y)
                        .speed(1.0)
                        .prefix("Y ")
                        .suffix(" m"),
                )
                .on_hover_text("世界 Y 坐标");
                ui.add(
                    DragValue::new(&mut camera.translation.z)
                        .speed(1.0)
                        .prefix("Z ")
                        .suffix(" m"),
                )
                .on_hover_text("世界 Z 坐标");
            });
            ui.end_row();

            ui.label("朝向");
            ui.horizontal(|ui| {
                let mut yaw = camera.yaw.to_degrees();
                let mut pitch = camera.pitch.to_degrees();
                let mut roll = camera.roll.to_degrees();
                let yaw_changed = ui
                    .add(
                        DragValue::new(&mut yaw)
                            .speed(0.5)
                            .suffix("°")
                            .clamp_existing_to_range(false),
                    )
                    .on_hover_text("偏航 Yaw")
                    .changed();
                let pitch_changed = ui
                    .add(
                        DragValue::new(&mut pitch)
                            .speed(0.5)
                            .suffix("°")
                            .range(0.0..=180.0)
                            .clamp_existing_to_range(false),
                    )
                    .on_hover_text("俯仰 Pitch")
                    .changed();
                let roll_changed = ui
                    .add(
                        DragValue::new(&mut roll)
                            .speed(0.5)
                            .suffix("°")
                            .range(-180.0..=180.0)
                            .clamp_existing_to_range(false),
                    )
                    .on_hover_text("翻滚 Roll")
                    .changed();
                if yaw_changed {
                    camera.yaw = yaw.rem_euclid(360.0).to_radians();
                }
                if pitch_changed {
                    camera.pitch = pitch.to_radians();
                }
                if roll_changed {
                    camera.roll = roll.to_radians();
                }
            });
            ui.end_row();
        });

    let mut fov = camera.fov.to_degrees();
    ui.horizontal(|ui| {
        ui.label("视野");
        ui.spacing_mut().slider_width = 190.0;
        if ui
            .add(Slider::new(&mut fov, 20.0..=140.0).suffix("°"))
            .changed()
        {
            camera.fov = fov.to_radians();
        }
    });

    ui.horizontal(|ui| {
        ui.label("速度");
        ui.add(
            DragValue::new(&mut camera.speed)
                .speed(10.0)
                .range(1.0..=100_000.0)
                .suffix(" m/s"),
        );
    });
}

fn time_ui(ui: &mut Ui, control: &mut Control) {
    let mut seconds = control.time.as_secs_f64() % DAY_SECONDS;
    ui.horizontal(|ui| {
        ui.label("时刻");
        ui.monospace(format_time(seconds));
    });
    ui.spacing_mut().slider_width = 280.0;
    if ui
        .add(Slider::new(&mut seconds, 0.0..=DAY_SECONDS).show_value(false))
        .changed()
    {
        control.time = Duration::from_secs_f64(seconds);
    }

    ui.horizontal(|ui| {
        ui.checkbox(&mut control.time_paused, "冻结时间");
        ui.separator();
        ui.label("倍率");
        ui.add(
            DragValue::new(&mut control.time_scale)
                .speed(0.1)
                .range(0.1..=10.0)
                .suffix("×"),
        );
    });
    ui.small("Z / X 手动拨动时间 · Shift 加速");
}

fn clouds_ui(ui: &mut Ui, clouds: &mut Clouds) {
    ui.spacing_mut().slider_width = 150.0;
    ui.add(
        Slider::new(&mut clouds.coverage, 0.0..=1.0)
            .text("覆盖")
            .fixed_decimals(2),
    );
    ui.add(
        Slider::new(&mut clouds.density, 0.1..=3.0)
            .text("密度")
            .fixed_decimals(2),
    );
    ui.horizontal(|ui| {
        ui.label("云底");
        ui.add(
            DragValue::new(&mut clouds.base)
                .speed(50.0)
                .range(500.0..=5000.0)
                .suffix(" m"),
        );
        ui.label("云顶");
        ui.add(
            DragValue::new(&mut clouds.top)
                .speed(50.0)
                .range(3000.0..=12000.0)
                .suffix(" m"),
        );
    });
    if clouds.top < clouds.base + 500.0 {
        ui.colored_label(Color32::from_rgb(235, 200, 110), "云顶至少比云底高 500 m");
    }
    ui.add(
        Slider::new(&mut clouds.wind, 0.0..=60.0)
            .text("风速")
            .suffix(" m/s"),
    );
    ui.add(
        Slider::new(&mut clouds.quality, 0.0..=1.0)
            .text("质量")
            .fixed_decimals(2),
    );
    let view_steps = 24 + (clouds.quality * 56.0) as u32;
    let light_steps = 4 + (clouds.quality * 4.0) as u32;
    ui.small(format!("{view_steps} 步 · {light_steps} 光照步"));
}

fn format_time(seconds: f64) -> String {
    let total = (seconds + 6. * 60. * 60.).max(0.0) as u64;
    format!(
        "{:02}:{:02}:{:02}",
        (total / 3600) % 24,
        (total / 60) % 60,
        total % 60
    )
}

fn fresh_raw_input(pixels_per_point: f32) -> RawInput {
    let mut raw_input = RawInput::default();
    if let Some(viewport) = raw_input.viewports.get_mut(&ViewportId::ROOT) {
        viewport.native_pixels_per_point = Some(pixels_per_point);
    }
    raw_input
}

fn install_fonts(context: &Context) {
    for path in CJK_FONT_CANDIDATES {
        if !Path::new(path).is_file() {
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };

        let mut fonts = FontDefinitions::default();
        fonts
            .font_data
            .insert("cjk".to_owned(), Arc::new(FontData::from_owned(bytes)));
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            if let Some(fonts) = fonts.families.get_mut(&family) {
                fonts.push("cjk".to_owned());
            }
        }
        context.set_fonts(fonts);
        log::debug!("debug ui: loaded CJK font from {path}");
        return;
    }

    log::warn!("debug ui: no CJK font found, Chinese labels may not render");
}

// winit -> egui // glues

fn egui_modifiers(modifiers: &winit::event::Modifiers) -> Modifiers {
    let state = modifiers.state();
    let ctrl = state.control_key();
    let mac_cmd = cfg!(target_os = "macos") && state.meta_key();
    let command = if cfg!(target_os = "macos") {
        mac_cmd
    } else {
        ctrl
    };
    Modifiers {
        alt: state.alt_key(),
        ctrl,
        shift: state.shift_key(),
        mac_cmd,
        command,
    }
}

fn pointer_button(button: MouseButton) -> Option<PointerButton> {
    Some(match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Back => PointerButton::Extra1,
        MouseButton::Forward => PointerButton::Extra2,
        _ => return None,
    })
}

fn egui_touch_phase(phase: WinitTouchPhase) -> TouchPhase {
    match phase {
        WinitTouchPhase::Started => TouchPhase::Start,
        WinitTouchPhase::Moved => TouchPhase::Move,
        WinitTouchPhase::Ended => TouchPhase::End,
        WinitTouchPhase::Cancelled => TouchPhase::Cancel,
    }
}

fn egui_key(code: KeyCode) -> Option<Key> {
    use KeyCode::*;
    Some(match code {
        Backquote => Key::Backtick,
        Backslash => Key::Backslash,
        BracketLeft => Key::OpenBracket,
        BracketRight => Key::CloseBracket,
        Comma => Key::Comma,
        Digit0 => Key::Num0,
        Digit1 => Key::Num1,
        Digit2 => Key::Num2,
        Digit3 => Key::Num3,
        Digit4 => Key::Num4,
        Digit5 => Key::Num5,
        Digit6 => Key::Num6,
        Digit7 => Key::Num7,
        Digit8 => Key::Num8,
        Digit9 => Key::Num9,
        Equal => Key::Equals,
        KeyA => Key::A,
        KeyB => Key::B,
        KeyC => Key::C,
        KeyD => Key::D,
        KeyE => Key::E,
        KeyF => Key::F,
        KeyG => Key::G,
        KeyH => Key::H,
        KeyI => Key::I,
        KeyJ => Key::J,
        KeyK => Key::K,
        KeyL => Key::L,
        KeyM => Key::M,
        KeyN => Key::N,
        KeyO => Key::O,
        KeyP => Key::P,
        KeyQ => Key::Q,
        KeyR => Key::R,
        KeyS => Key::S,
        KeyT => Key::T,
        KeyU => Key::U,
        KeyV => Key::V,
        KeyW => Key::W,
        KeyX => Key::X,
        KeyY => Key::Y,
        KeyZ => Key::Z,
        Minus => Key::Minus,
        Period => Key::Period,
        Quote => Key::Quote,
        Semicolon => Key::Semicolon,
        Slash => Key::Slash,
        Space => Key::Space,
        Tab => Key::Tab,
        Enter => Key::Enter,
        Backspace => Key::Backspace,
        Delete => Key::Delete,
        Insert => Key::Insert,
        Home => Key::Home,
        End => Key::End,
        PageUp => Key::PageUp,
        PageDown => Key::PageDown,
        ArrowUp => Key::ArrowUp,
        ArrowDown => Key::ArrowDown,
        ArrowLeft => Key::ArrowLeft,
        ArrowRight => Key::ArrowRight,
        Escape => Key::Escape,
        F1 => Key::F1,
        F2 => Key::F2,
        F3 => Key::F3,
        F4 => Key::F4,
        F5 => Key::F5,
        F6 => Key::F6,
        F7 => Key::F7,
        F8 => Key::F8,
        F9 => Key::F9,
        F10 => Key::F10,
        F11 => Key::F11,
        F12 => Key::F12,
        _ => return None,
    })
}
