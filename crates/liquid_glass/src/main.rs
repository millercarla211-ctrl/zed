//! Liquid Glass in Rust — wgpu + winit + imgui
//!
//! Faithful port of OverShifted/LiquidGlass (C++/OpenGL).
//! The effect runs on the GPU. ImGui provides the control panel.

mod ui_state;

use bytemuck::{Pod, Zeroable};
use imgui::Context as ImGuiContext;
use imgui_wgpu::RendererConfig;
use imgui_winit_support::WinitPlatform;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use ui_state::UiState;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

// ═══════════════════════════════════════════════════════════════════
// Uniform buffer — must match WGSL struct layout EXACTLY.
// wgpu requires 16-byte alignment for uniform structs.
// Each "row" below is one vec4 (16 bytes).
// ═══════════════════════════════════════════════════════════════════

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    // Row 0: vec4
    resolution: [f32; 2], // Window size (pixels)
    glass_pos: [f32; 2],  // Glass center (pixels)

    // Row 1: vec4
    glass_size: [f32; 2], // Glass width, height (pixels)
    time: f32,
    power_factor: f32, // Superellipse power

    // Row 2: vec4
    a: f32, // f(x) parameter a
    b: f32, // f(x) parameter b
    c: f32, // f(x) parameter c
    d: f32, // f(x) parameter d

    // Row 3: vec4
    f_power: f32,     // Distortion power
    noise: f32,       // Noise intensity
    glow_weight: f32, // Glow multiplier
    glow_edge0: f32,  // Glow smoothstep inner

    // Row 4: vec4
    glow_edge1: f32,           // Glow smoothstep outer
    glow_bias: f32,            // Glow bias
    chromatic_aberration: f32, // Chromatic aberration strength
    aberration_samples: f32,   // Number of samples (as float for alignment)
}

impl Uniforms {
    fn from_state(state: &UiState, resolution: [f32; 2], time: f32) -> Self {
        Self {
            resolution,
            glass_pos: state.position,
            glass_size: [state.glass_width_px(), state.glass_height_px()],
            time,
            power_factor: state.power_factor,
            a: state.a,
            b: state.b,
            c: state.c,
            d: state.d,
            f_power: state.f_power,
            noise: state.noise,
            glow_weight: state.glow_weight,
            glow_edge0: state.glow_edge0,
            glow_edge1: state.glow_edge1,
            glow_bias: state.glow_bias,
            chromatic_aberration: state.chromatic_aberration,
            aberration_samples: state.aberration_samples as f32,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// GPU State — all wgpu resources
// ═══════════════════════════════════════════════════════════════════

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // Main liquid glass pipeline
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,

    // Background texture resources
    bg_texture: wgpu::Texture,
    bg_sampler: wgpu::Sampler,

    // ImGui
    imgui: ImGuiContext,
    imgui_platform: WinitPlatform,
    imgui_renderer: imgui_wgpu::Renderer,

    // State
    ui_state: UiState,
    start_time: Instant,
    last_frame: Instant,
    frame_times: Vec<f32>, // Rolling window for FPS smoothing

    // Keyboard movement state
    velocity_multiplier: f32,
    camera_velocity_multiplier: f32,
    pressed_keys: HashSet<winit::keyboard::KeyCode>,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        // ── wgpu setup ──
        let instance = wgpu::Instance::default();
        let window_for_imgui = window.clone();
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // ── Load background texture ──
        let bg_texture = Self::create_background_texture(&device, &queue, 0);
        let bg_view = bg_texture.create_view(&Default::default());
        let bg_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ── Uniform buffer ──
        let ui_state = UiState::default();
        let uniforms = Uniforms::from_state(&ui_state, [width as f32, height as f32], 0.0);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ── Bind group layout ──
        // Binding 0: Uniforms
        // Binding 1: Background texture
        // Binding 2: Background sampler
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&bg_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&bg_sampler),
                },
            ],
        });

        // ── Shader + Pipeline ──
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Liquid Glass Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/liquid_glass.wgsl").into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // ── ImGui setup ──
        let mut imgui = ImGuiContext::create();
        imgui.set_ini_filename(None);
        imgui.style_mut().use_dark_colors();

        let mut imgui_platform = WinitPlatform::new(&mut imgui);
        imgui_platform.attach_window(
            imgui.io_mut(),
            &window_for_imgui,
            imgui_winit_support::HiDpiMode::Default,
        );

        let imgui_renderer = imgui_wgpu::Renderer::new(
            &mut imgui,
            &device,
            &queue,
            RendererConfig {
                texture_format: format,
                ..Default::default()
            },
        );

        Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            bg_texture,
            bg_sampler,
            imgui,
            imgui_platform,
            imgui_renderer,
            ui_state: UiState {
                position: [width as f32 / 2.0, height as f32 / 2.0],
                ..Default::default()
            },
            start_time: Instant::now(),
            last_frame: Instant::now(),
            frame_times: Vec::with_capacity(60),
            velocity_multiplier: 1.0,
            camera_velocity_multiplier: 1.0,
            pressed_keys: HashSet::new(),
        }
    }

    fn create_background_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bg_id: usize,
    ) -> wgpu::Texture {
        // Try to load from file using the background ID
        let path = ui_state::BACKGROUND_PATHS[bg_id];
        let (w, h, data) = match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                eprintln!("Loaded background {}: {}x{}", path, w, h);
                (w, h, rgba.into_raw())
            }
            Err(_) => {
                eprintln!("Failed to load {}, generating procedural", path);
                Self::generate_procedural_background()
            }
        };

        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Background"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            size,
        );

        texture
    }

    fn generate_procedural_background() -> (u32, u32, Vec<u8>) {
        use std::f32::consts::{PI, TAU};

        let w = 1280u32;
        let h = 720u32;
        let mut data = Vec::with_capacity((w * h * 4) as usize);

        for y in 0..h {
            for x in 0..w {
                let u = x as f32 / w as f32;
                let v = y as f32 / h as f32;

                // Vibrant multi-hue gradient with geometric patterns
                let r =
                    (0.5 + 0.3 * (u * TAU + 0.0).sin() + 0.2 * ((u * 15.0 + v * 10.0) * 1.5).cos())
                        .clamp(0.05, 0.95);
                let g = (0.4
                    + 0.3 * (v * TAU + 2.09).sin()
                    + 0.15 * ((u * 12.0 - v * 8.0) * 1.3).sin())
                .clamp(0.05, 0.95);
                let b = (0.6
                    + 0.3 * ((u + v) * PI + 4.18).sin()
                    + 0.2 * ((u * 8.0 + v * 12.0) * 1.7).sin())
                .clamp(0.05, 0.95);

                // Subtle checkerboard
                let cx = (u * 40.0).floor() as i32;
                let cy = (v * 40.0).floor() as i32;
                let checker = if (cx + cy) % 2 == 0 { 0.95 } else { 1.0 };

                // Circle pattern
                let du = u - 0.5;
                let dv = v - 0.5;
                let rings = (0.5 + 0.5 * ((du * du + dv * dv).sqrt() * 20.0).sin()) * 0.1 + 0.9;

                let brightness = checker * rings;

                data.push((r * brightness * 255.0) as u8);
                data.push((g * brightness * 255.0) as u8);
                data.push((b * brightness * 255.0) as u8);
                data.push(255);
            }
        }

        (w, h, data)
    }

    fn reload_background(&mut self, bg_id: usize) {
        self.bg_texture = Self::create_background_texture(&self.device, &self.queue, bg_id);
        let bg_view = self.bg_texture.create_view(&Default::default());

        // Recreate bind group with new texture
        self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main BG"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&bg_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.bg_sampler),
                },
            ],
        });
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        let w = new_size.width.max(1);
        let h = new_size.height.max(1);
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
    }

    fn handle_keyboard_input(&mut self, delta_time: f32) {
        use winit::keyboard::KeyCode;

        // Track if any keys are pressed for velocity multiplier
        let mut glass_key_pressed = false;
        let mut camera_key_pressed = false;

        // WASD - Move glass position (only if not in mouse control mode)
        if !self.ui_state.mouse_control {
            if self.pressed_keys.contains(&KeyCode::KeyW) {
                self.ui_state.position[1] -= delta_time
                    * self.velocity_multiplier
                    * self.ui_state.velocity
                    * self.ui_state.pixel_scale;
                glass_key_pressed = true;
            }
            if self.pressed_keys.contains(&KeyCode::KeyS) {
                self.ui_state.position[1] += delta_time
                    * self.velocity_multiplier
                    * self.ui_state.velocity
                    * self.ui_state.pixel_scale;
                glass_key_pressed = true;
            }
            if self.pressed_keys.contains(&KeyCode::KeyD) {
                self.ui_state.position[0] += delta_time
                    * self.velocity_multiplier
                    * self.ui_state.velocity
                    * self.ui_state.pixel_scale;
                glass_key_pressed = true;
            }
            if self.pressed_keys.contains(&KeyCode::KeyA) {
                self.ui_state.position[0] -= delta_time
                    * self.velocity_multiplier
                    * self.ui_state.velocity
                    * self.ui_state.pixel_scale;
                glass_key_pressed = true;
            }
        }

        // Arrow keys - Move camera
        if self.pressed_keys.contains(&KeyCode::ArrowUp) {
            self.ui_state.camera_position[1] +=
                delta_time * self.camera_velocity_multiplier * self.ui_state.camera_velocity;
            camera_key_pressed = true;
        }
        if self.pressed_keys.contains(&KeyCode::ArrowDown) {
            self.ui_state.camera_position[1] -=
                delta_time * self.camera_velocity_multiplier * self.ui_state.camera_velocity;
            camera_key_pressed = true;
        }
        if self.pressed_keys.contains(&KeyCode::ArrowRight) {
            self.ui_state.camera_position[0] +=
                delta_time * self.camera_velocity_multiplier * self.ui_state.camera_velocity;
            camera_key_pressed = true;
        }
        if self.pressed_keys.contains(&KeyCode::ArrowLeft) {
            self.ui_state.camera_position[0] -=
                delta_time * self.camera_velocity_multiplier * self.ui_state.camera_velocity;
            camera_key_pressed = true;
        }

        // Q/A - Adjust power factor
        if self.pressed_keys.contains(&KeyCode::KeyQ) {
            self.ui_state.power_factor += delta_time * 2.0;
            self.ui_state.power_factor = self.ui_state.power_factor.min(6.0);
        }
        if self.pressed_keys.contains(&KeyCode::KeyE) {
            self.ui_state.power_factor -= delta_time * 2.0;
            self.ui_state.power_factor = self.ui_state.power_factor.max(1.001);
        }

        // Update velocity multipliers
        if glass_key_pressed {
            self.velocity_multiplier += 1.0 * delta_time;
        } else {
            self.velocity_multiplier -= 3.0 * delta_time;
        }

        if camera_key_pressed {
            self.camera_velocity_multiplier += 1.0 * delta_time;
        } else {
            self.camera_velocity_multiplier -= 3.0 * delta_time;
        }

        self.velocity_multiplier = self.velocity_multiplier.clamp(0.0, 1.0);
        self.camera_velocity_multiplier = self.camera_velocity_multiplier.clamp(0.0, 1.0);
    }

    fn update_and_render(&mut self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        let elapsed = self.start_time.elapsed().as_secs_f32();

        // Update rolling FPS calculation (keep last 60 frames)
        self.frame_times.push(delta_time);
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }

        // Calculate average FPS from rolling window
        let avg_delta = if !self.frame_times.is_empty() {
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
        } else {
            delta_time
        };
        let fps = if avg_delta > 0.0 {
            1.0 / avg_delta
        } else {
            0.0
        };

        // Track previous background ID to detect changes
        static mut PREV_BG_ID: usize = 0;
        let current_bg = self.ui_state.current_bg;
        unsafe {
            if current_bg != PREV_BG_ID {
                self.reload_background(current_bg);
                PREV_BG_ID = current_bg;
            }
        }

        // Handle keyboard movement (WASD for glass, arrows for camera)
        self.handle_keyboard_input(delta_time);

        // ── Update uniforms from UI state ──
        let uniforms = Uniforms::from_state(
            &self.ui_state,
            [self.config.width as f32, self.config.height as f32],
            elapsed,
        );
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // ── ImGui frame ──
        self.imgui_platform
            .prepare_frame(self.imgui.io_mut(), window)
            .ok();
        let ui = self.imgui.frame();

        // ── Draw ImGui controls (matching C++ exactly) ──
        ui.window("Settings")
            .size([340.0, 750.0], imgui::Condition::FirstUseEver)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                // ── Background Selector ──
                let bg_names = ui_state::BACKGROUNDS;
                let current_bg_name = bg_names[self.ui_state.current_bg];

                if let Some(_token) = ui.begin_combo("Background", current_bg_name) {
                    for (i, name) in bg_names.iter().enumerate() {
                        let is_selected = i == self.ui_state.current_bg;
                        if ui.selectable_config(name).selected(is_selected).build() {
                            self.ui_state.current_bg = i;
                        }
                        if is_selected {
                            ui.set_item_default_focus();
                        }
                    }
                }

                ui.dummy([0.0, 5.0]);

                // ── Glass Variant Selector ──
                let variant_names = ui_state::GLASS_VARIANTS;
                let current_variant_name = variant_names[self.ui_state.glass_variant];

                if let Some(_token) = ui.begin_combo("Glass Style", current_variant_name) {
                    for (i, name) in variant_names.iter().enumerate() {
                        let is_selected = i == self.ui_state.glass_variant;
                        if ui.selectable_config(name).selected(is_selected).build() {
                            self.ui_state.glass_variant = i;
                            self.ui_state.apply_variant_size();
                        }
                        if is_selected {
                            ui.set_item_default_focus();
                        }
                    }
                }

                ui.checkbox("Move with mouse", &mut self.ui_state.mouse_control);

                ui.dummy([0.0, 8.0]);

                // ── Shape ──
                if ui.collapsing_header("Shape", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                    ui.slider_config("Power", 1.001, 6.0)
                        .display_format("%.3f")
                        .build(&mut self.ui_state.power_factor);
                    ui.slider_config("Width", 0.0, 10.0)
                        .display_format("%.2f")
                        .build(&mut self.ui_state.width);
                    ui.slider_config("Height", 0.0, 10.0)
                        .display_format("%.2f")
                        .build(&mut self.ui_state.height);
                }

                ui.dummy([0.0, 5.0]);

                // ── Chromatic Aberration (Apple Liquid Glass) ──
                if ui.collapsing_header("Chromatic Aberration", imgui::TreeNodeFlags::DEFAULT_OPEN)
                {
                    ui.text("Apple Liquid Glass Effect");
                    ui.slider_config("Aberration", 0.0, 0.02)
                        .display_format("%.4f")
                        .build(&mut self.ui_state.chromatic_aberration);
                    let mut samples = self.ui_state.aberration_samples;
                    if ui
                        .slider_config("Quality", 1, 8)
                        .display_format("%d samples")
                        .build(&mut samples)
                    {
                        self.ui_state.aberration_samples = samples;
                    }
                    ui.text_disabled("Separates RGB colors for prismatic effect");
                }

                ui.dummy([0.0, 5.0]);

                // ── Blur & Noise ──
                if ui.collapsing_header("Blur & Noise", imgui::TreeNodeFlags::empty()) {
                    let mut iters = self.ui_state.blur_iterations as i32;
                    if ui
                        .slider_config("Blur Iters", 0, 10)
                        .display_format("%d")
                        .build(&mut iters)
                    {
                        self.ui_state.blur_iterations = iters as u32;
                    }
                    ui.slider_config("Blur Radius", 0.0, 10.0)
                        .display_format("%.2f")
                        .build(&mut self.ui_state.blur_radius);
                    ui.slider_config("Blur downscale", 0.1, 1.0)
                        .display_format("%.2f")
                        .build(&mut self.ui_state.blur_downscale);
                    ui.slider_config("Noise", 0.0, 0.3)
                        .display_format("%.3f")
                        .build(&mut self.ui_state.noise);
                }

                ui.dummy([0.0, 5.0]);

                // ── Refraction ──
                if ui.collapsing_header("Refraction", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                    ui.text("f(x) = 1 - b (ce)^(-dx-a)");
                    ui.slider_config("f(x) Power", -1.5, 6.0)
                        .display_format("%.2f")
                        .build(&mut self.ui_state.f_power);
                    ui.slider_config("a", 0.0, 5.0)
                        .display_format("%.2f")
                        .build(&mut self.ui_state.a);
                    ui.slider_config("b", 0.0, 6.0)
                        .display_format("%.2f")
                        .build(&mut self.ui_state.b);
                    ui.slider_config("c", 0.0, 6.0)
                        .display_format("%.2f")
                        .build(&mut self.ui_state.c);
                    ui.slider_config("d", 0.0, 10.0)
                        .display_format("%.2f")
                        .build(&mut self.ui_state.d);
                }

                ui.dummy([0.0, 5.0]);

                // ── Glow ──
                if ui.collapsing_header("Glow", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                    ui.slider_config("Glow weight", -1.0, 1.0)
                        .display_format("%.3f")
                        .build(&mut self.ui_state.glow_weight);
                    ui.slider_config("Glow bias", -1.0, 1.0)
                        .display_format("%.3f")
                        .build(&mut self.ui_state.glow_bias);
                    ui.slider_config("Glow edge0", -1.0, 1.0)
                        .display_format("%.3f")
                        .build(&mut self.ui_state.glow_edge0);
                    ui.slider_config("Glow edge1", -1.0, 1.0)
                        .display_format("%.3f")
                        .build(&mut self.ui_state.glow_edge1);
                }

                ui.dummy([0.0, 5.0]);

                // ── FPS Counter ──
                ui.text(format!("{:.0} FPS", fps));

                // ── Reload Shader Button ──
                if ui.button("Reload shader") {
                    // In Rust/wgpu, we'd need to recreate the pipeline
                    // For now, this is a placeholder
                }

                // ── VSync Toggle ──
                let mut vsync = self.config.present_mode != wgpu::PresentMode::Immediate;
                if ui.checkbox("VSync", &mut vsync) {
                    self.config.present_mode = if vsync {
                        wgpu::PresentMode::AutoVsync
                    } else {
                        wgpu::PresentMode::Immediate
                    };
                    self.surface.configure(&self.device, &self.config);
                }

                ui.dummy([0.0, 20.0]);

                // ── Credits ──
                let credits = ui_state::BACKGROUND_CREDITS[self.ui_state.current_bg];
                if !credits.is_empty() {
                    ui.text_disabled(credits);
                }
            });

        // ── Get surface texture ──
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        // ── Render liquid glass effect ──
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Liquid Glass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1); // Fullscreen triangle
        }

        // ── Pass 2: ImGui overlay ──
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ImGui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear — overlay on top
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.imgui_renderer
                .render(self.imgui.render(), &self.queue, &self.device, &mut pass)
                .expect("ImGui render failed");
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// Application Handler
// ═══════════════════════════════════════════════════════════════════

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = WindowAttributes::default()
                .with_title("🧊 Liquid Glass — Rust + wgpu")
                .with_inner_size(PhysicalSize::new(1024u32, 768u32));

            let window = Arc::new(event_loop.create_window(attrs).unwrap());
            let gpu = pollster::block_on(GpuState::new(window.clone()));
            self.gpu = Some(gpu);
            self.window = Some(window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let (Some(window), Some(gpu)) = (self.window.as_ref(), self.gpu.as_mut()) else {
            return;
        };

        // Forward events to ImGui
        let event_for_imgui: winit::event::Event<()> = winit::event::Event::WindowEvent {
            window_id: window.id(),
            event: event.clone(),
        };
        gpu.imgui_platform
            .handle_event(gpu.imgui.io_mut(), window, &event_for_imgui);

        // Don't process events ImGui captured
        let io = gpu.imgui.io();
        let imgui_wants_mouse = io.want_capture_mouse;
        let imgui_wants_kb = io.want_capture_keyboard;

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                gpu.resize(size);
                window.request_redraw();
            }

            WindowEvent::CursorMoved { position, .. } if !imgui_wants_mouse => {
                if gpu.ui_state.mouse_control {
                    gpu.ui_state.position = [position.x as f32, position.y as f32];
                }
                window.request_redraw();
            }

            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if !imgui_wants_kb => {
                if let PhysicalKey::Code(keycode) = key_event.physical_key {
                    match key_event.state {
                        ElementState::Pressed => {
                            // Handle special keys
                            match keycode {
                                KeyCode::Escape => event_loop.exit(),
                                KeyCode::Space => {
                                    // Reset to defaults
                                    let current_pos = gpu.ui_state.position;
                                    gpu.ui_state = UiState {
                                        position: current_pos,
                                        ..Default::default()
                                    };
                                }
                                _ => {
                                    gpu.pressed_keys.insert(keycode);
                                }
                            }
                        }
                        ElementState::Released => {
                            gpu.pressed_keys.remove(&keycode);
                        }
                    }
                }
                window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                match gpu.update_and_render(window) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        gpu.resize(window.inner_size());
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("{:?}", e),
                }
                // Continuous redraw for animation
                window.request_redraw();
            }

            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Entry Point
// ═══════════════════════════════════════════════════════════════════

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    eprintln!("Starting Liquid Glass...");
    eprintln!("Controls:");
    eprintln!("  WASD = Move glass position");
    eprintln!("  Arrow keys = Move camera");
    eprintln!("  Q/E = Adjust power factor");
    eprintln!("  Mouse = Reposition glass (when 'Move with mouse' enabled)");
    eprintln!("  Space = Reset to defaults");
    eprintln!("  Esc = Quit");

    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        window: None,
        gpu: None,
    };
    event_loop.run_app(&mut app).unwrap();
}
