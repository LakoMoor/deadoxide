//! Zomboid-rs client. `zomboid-client <root> [cell] [--shot out.png [cx cy zoom w h]]`
mod light;
mod scene;
mod sim;
use scene::*;
use sim::{Player, World, TICK_HZ};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

/// Measured against the real game (tests/golden/spawn_rosewood): camera centre sits 56 px above the player feet at zoom 1. One sample only.
const CAM_OVER_FEET: f32 = 56.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CamU {
    center: [f32; 2],
    scale: [f32; 2],
    /// x = ambient multiplier on the world colour (1.0 = unlit), yzw unused
    light: [f32; 4],
}

#[derive(Clone, Copy)]
struct Camera {
    center: [f32; 2],
    zoom: f32,
    max_level: usize,
    ambient: f32,
}

struct Gfx {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    cam_buf: wgpu::Buffer,
    inst_buf: wgpu::Buffer,
    player_buf: wgpu::Buffer,
    level_ranges: Vec<u32>,
    clear: [f64; 3],
}

impl Gfx {
    fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        scene: &Scene,
    ) -> Self {
        let a = &scene.atlas;
        let tex = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("atlas"),
                size: wgpu::Extent3d {
                    width: a.width,
                    height: a.height,
                    depth_or_array_layers: a.layers,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &a.rgba,
        );
        let view = tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let samp = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let cam_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cam"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let inst_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("inst"),
            contents: bytemuck::cast_slice(&scene.instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
                        view_dimension: wgpu::TextureViewDimension::D2Array,
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
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cam_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&samp),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprites"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 36,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attrs,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let player_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("player"),
            size: (std::mem::size_of::<Instance>() * 2048) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            device,
            queue,
            pipeline,
            bind,
            cam_buf,
            inst_buf,
            player_buf,
            level_ranges: scene.level_ranges.clone(),
            clear: scene.clear.map(|c| c as f64),
        }
    }

    fn draw(&self, view: &wgpu::TextureView, size: (u32, u32), cam: Camera, markers: &[Instance]) {
        let markers = &markers[..markers.len().min(2048)];
        if !markers.is_empty() {
            self.queue
                .write_buffer(&self.player_buf, 0, bytemuck::cast_slice(markers));
        }
        let u = CamU {
            center: cam.center,
            scale: [
                2.0 * cam.zoom / size.0 as f32,
                2.0 * cam.zoom / size.1 as f32,
            ],
            light: [cam.ambient, 0.0, 0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.cam_buf, 0, bytemuck::bytes_of(&u));
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear[0],
                            g: self.clear[1],
                            b: self.clear[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_vertex_buffer(0, self.inst_buf.slice(..));
            let top = (cam.max_level + 1).min(self.level_ranges.len() - 1);
            pass.draw(0..4, 0..self.level_ranges[top]);
            if !markers.is_empty() {
                pass.set_vertex_buffer(0, self.player_buf.slice(..));
                pass.draw(0..4, 0..markers.len() as u32);
            }
        }
        self.queue.submit([enc.finish()]);
    }
}

fn init_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'static>>,
) -> anyhow::Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: surface,
        ..Default::default()
    }))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    Ok((adapter, device, queue))
}

/// Offscreen render + readback to PNG (same draw path as the window).
fn screenshot(
    scene: &Scene,
    cam: Camera,
    size: (u32, u32),
    player: Option<(f32, f32, i32)>,
    extra: Vec<Instance>,
    out: &str,
) -> anyhow::Result<()> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (_a, device, queue) = init_device(&instance, None)?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let gfx = Gfx::new(device, queue, format, scene);
    let extent = wgpu::Extent3d {
        width: size.0,
        height: size.1,
        depth_or_array_layers: 1,
    };
    let tex = gfx.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    let mut mk: Vec<Instance> = extra;
    mk.extend(player.map(|(x, y, z)| scene.marker("__player", x, y, z)));
    gfx.draw(&view, size, cam, &mk);
    let bpr = (size.0 * 4).div_ceil(256) * 256;
    let buf = gfx.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (bpr * size.1) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = gfx.device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        tex.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bpr),
                rows_per_image: Some(size.1),
            },
        },
        extent,
    );
    gfx.queue.submit([enc.finish()]);
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gfx.device.poll(wgpu::PollType::wait_indefinitely())?;
    let data = slice.get_mapped_range()?;
    let mut rgba = Vec::with_capacity((size.0 * size.1 * 4) as usize);
    for row in 0..size.1 {
        rgba.extend_from_slice(&data[(row * bpr) as usize..(row * bpr + size.0 * 4) as usize]);
    }
    let mut enc = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(out)?),
        size.0,
        size.1,
    );
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&rgba)?;
    println!("screenshot -> {out}");
    Ok(())
}

struct Live {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gfx: Gfx,
}

struct App {
    scene: Scene,
    live: Option<Live>,
    cam: Camera,
    world: World,
    lua: zomboid_lua::events::EventRuntime,
    acc: f32,
    keys: std::collections::HashSet<KeyCode>,
    drag: Option<(f64, f64)>,
    last: std::time::Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.live.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("zomboid-rs")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
        let window = Arc::new(el.create_window(attrs).unwrap());
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone()).unwrap();
        let (adapter, device, queue) = init_device(&instance, Some(&surface)).expect("gpu");
        let caps = surface.get_capabilities(&adapter);
        // non-sRGB so atlas bytes pass through unchanged (matches the CPU reference)
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let sz = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: sz.width.max(1),
            height: sz.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: Default::default(),
        };
        surface.configure(&device, &config);
        let gfx = Gfx::new(device, queue, format, &self.scene);
        self.live = Some(Live {
            window,
            surface,
            config,
            gfx,
        });
        self.last = std::time::Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        let Some(l) = &mut self.live else { return };
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                l.config.width = s.width;
                l.config.height = s.height;
                l.surface.configure(&l.gfx.device, &l.config);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(k) = event.physical_key {
                    if event.state == ElementState::Pressed {
                        self.keys.insert(k);
                        let top = self.scene.level_ranges.len() - 2;
                        match k {
                            KeyCode::Escape => el.exit(),
                            KeyCode::PageUp => {
                                self.cam.max_level = (self.cam.max_level + 1).min(top)
                            }
                            KeyCode::PageDown => {
                                self.cam.max_level = self.cam.max_level.saturating_sub(1)
                            }
                            _ => {}
                        }
                    } else {
                        self.keys.remove(&k);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                self.cam.zoom = (self.cam.zoom * 1.1f32.powf(y)).clamp(0.05, 4.0);
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.drag = if state == ElementState::Pressed {
                    Some((f64::NAN, 0.0))
                } else {
                    None
                };
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(d) = &mut self.drag {
                    if !d.0.is_nan() {
                        self.cam.center[0] -= (position.x - d.0) as f32 / self.cam.zoom;
                        self.cam.center[1] -= (position.y - d.1) as f32 / self.cam.zoom;
                    }
                    *d = (position.x, position.y);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let dt = (now - self.last).as_secs_f32();
                self.last = now;
                // WASD = screen directions; world axes are rotated 45 degrees in the isometric view
                let (mut sx, mut sy) = (0.0f32, 0.0f32);
                for k in &self.keys {
                    match k {
                        KeyCode::KeyW | KeyCode::ArrowUp => {
                            sx -= 1.0;
                            sy -= 1.0;
                        }
                        KeyCode::KeyS | KeyCode::ArrowDown => {
                            sx += 1.0;
                            sy += 1.0;
                        }
                        KeyCode::KeyA | KeyCode::ArrowLeft => {
                            sx -= 1.0;
                            sy += 1.0;
                        }
                        KeyCode::KeyD | KeyCode::ArrowRight => {
                            sx += 1.0;
                            sy -= 1.0;
                        }
                        _ => {}
                    }
                }
                // fixed-step simulation (30 Hz), render interpolation not implemented
                self.acc += dt.min(0.25);
                while self.acc >= 1.0 / TICK_HZ {
                    if let Err(error) =
                        advance_world(&mut self.world, &self.scene.collision, (sx, sy), &self.lua)
                    {
                        log::error!("Lua event dispatch failed: {error}");
                    }
                    self.acc -= 1.0 / TICK_HZ;
                }
                {
                    let c = iso(
                        self.world.player.x,
                        self.world.player.y,
                        self.world.player.z,
                    );
                    self.cam.center = [c[0], c[1] - CAM_OVER_FEET];
                }
                let mut markers: Vec<Instance> = self
                    .world
                    .zombies
                    .iter()
                    .map(|z| self.scene.marker("__zombie", z.x, z.y, self.world.player.z))
                    .collect();
                markers.push(self.scene.marker(
                    "__player",
                    self.world.player.x,
                    self.world.player.y,
                    self.world.player.z,
                ));
                use wgpu::CurrentSurfaceTexture as T;
                match l.surface.get_current_texture() {
                    T::Success(f) | T::Suboptimal(f) => {
                        let view = f.texture.create_view(&Default::default());
                        l.gfx
                            .draw(&view, (l.config.width, l.config.height), self.cam, &markers);
                        l.gfx.queue.present(f);
                    }
                    T::Outdated | T::Lost => l.surface.configure(&l.gfx.device, &l.config),
                    _ => {}
                }
                l.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let a: Vec<String> = std::env::args().collect();
    let root = std::path::PathBuf::from(a.get(1).map(String::as_str).unwrap_or(".."));
    // --save <save folder or players.db>: read a consistent committed player row, including WAL.
    let saved = a
        .iter()
        .position(|s| s == "--save")
        .map(|j| load_saved_player(&a[j + 1]))
        .transpose()?;
    let mut saved_time = None;
    let mut saved_day = None;
    if let Some(j) = a.iter().position(|s| s == "--save") {
        let save_path = std::path::Path::new(&a[j + 1]);
        let folder = if save_path.is_dir() {
            Some(save_path)
        } else {
            save_path.parent()
        };
        if let Some(path) = folder
            .map(|p| p.join("map_sand.bin"))
            .filter(|p| p.is_file())
        {
            match zomboid_content::sandbox::load_day_length(&path) {
                Ok(day) => {
                    println!(
                        "save day length: {} minutes (option {})",
                        day.minutes, day.option
                    );
                    saved_day = Some(day);
                }
                Err(e) => eprintln!("save day length unavailable: {e}"),
            }
        }
        if let Some(path) = folder.map(|p| p.join("map_t.bin")).filter(|p| p.is_file()) {
            match zomboid_content::save::load_game_time(&path) {
                Ok(t) => {
                    println!(
                        "save time: {:02}:{:02}, year {}, month index {}, day index {}",
                        t.hour(),
                        t.minutes(),
                        t.year,
                        t.month_index,
                        t.day_index
                    );
                    saved_time = Some(t);
                }
                Err(e) => eprintln!("save time unavailable: {e}"),
            }
        }
    }
    let cell = a
        .get(2)
        .filter(|s| !s.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| match &saved {
            Some(p) => {
                let (cx, cy) = zomboid_content::save::cell_of(p.x, p.y);
                format!("{cx}_{cy}")
            }
            None => "41_39".into(),
        });
    let (r0, r1) = cell
        .split_once(':')
        .unwrap_or((cell.as_str(), cell.as_str()));
    let pc = |s: &str| {
        let (x, y) = s.split_once('_').expect("cell like 41_39");
        (x.parse::<i32>().unwrap(), y.parse::<i32>().unwrap())
    };
    let ((x0, y0), (x1, y1)) = (pc(r0), pc(r1));
    let t0 = std::time::Instant::now();
    let mut scene = load_region(&root, x0, y0, x1, y1)?;
    println!(
        "cell {cell}: {} sprites, atlas {}x{}, unresolved {}, levels {}, loaded in {:.2}s",
        scene.instances.len(),
        scene.atlas.width,
        scene.atlas.height,
        scene.unresolved,
        scene.level_ranges.len() - 1,
        t0.elapsed().as_secs_f32()
    );
    let top = scene.level_ranges.len() - 2;
    let (gx, gy) = ((x0 * 256 + 116) as f32, (y0 * 256 + 116) as f32);
    let mut cam = Camera {
        center: [(gx - gy) * 64.0, (gx + gy) * 32.0],
        zoom: 0.5,
        max_level: top,
        ambient: 1.0,
    };
    if let Some(p) = &saved {
        let position = iso(p.x as f32, p.y as f32, p.z);
        cam.center = [position[0], position[1] - CAM_OVER_FEET];
        cam.max_level = (p.z - scene.min_level).clamp(0, top as i32) as usize;
    }
    if let Some(i) = a.iter().position(|s| s == "--shot") {
        let f = |n: usize, d: f32| a.get(i + n).and_then(|s| s.parse().ok()).unwrap_or(d);
        cam.center = [f(2, cam.center[0]), f(3, cam.center[1])];
        cam.zoom = f(4, 0.5);
        if let Some(j) = a.iter().position(|s| s == "--level") {
            cam.max_level = a[j + 1].parse().unwrap();
        }
        let pl: Option<(f32, f32, i32)> = a
            .iter()
            .position(|s| s == "--player")
            .map(|j| {
                let z = a
                    .iter()
                    .position(|s| s == "--player-z")
                    .and_then(|k| a.get(k + 1))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                (a[j + 1].parse().unwrap(), a[j + 2].parse().unwrap(), z)
            })
            .or_else(|| saved.as_ref().map(|p| (p.x as f32, p.y as f32, p.z)));
        // Night ambient on lit squares: game/port luma ratio is ~0.8 in the room interior on two real frames (docs/COMPATIBILITY.md, tools/ratio_map.mjs).
        if a.iter().any(|s| s == "--lit-room") {
            cam.ambient = 0.8;
        }
        if let Some(j) = a.iter().position(|s| s == "--ambient") {
            cam.ambient = a[j + 1].parse().unwrap_or(1.0);
        }
        if a.iter().any(|s| s == "--lit-room") {
            if let Some((px, py, pz)) = pl {
                println!(
                    "lit-room: {:?}, instances now {}",
                    scene.retain_rooms(
                        px,
                        py,
                        pz,
                        a.iter().any(|s| s == "--probe-rooms"),
                        a.iter()
                            .position(|s| s == "--los")
                            .and_then(|j| a[j + 1].parse().ok())
                    ),
                    scene.instances.len()
                );
            }
        }
        // --zones <Type>: outline that zone type from the map's objects.lua (debug overlay, tile box = 60x60 around the player/camera)
        let mut extra = vec![];
        if let Some(j) = a.iter().position(|s| s == "--zones") {
            let zs = zomboid_content::zones::parse_objects(&std::fs::read(
                root.join("media/maps/Muldraugh, KY/objects.lua"),
            )?)
            .map_err(|e| anyhow::anyhow!(e))?;
            let (px, py, pz) = pl.unwrap_or((gx, gy, 0));
            extra = scene.zone_dots(
                "__player",
                &zs,
                &a[j + 1],
                (
                    (px - 60.0) as i32,
                    (py - 60.0) as i32,
                    (px + 60.0) as i32,
                    (py + 60.0) as i32,
                ),
                pz,
                1.0,
            );
            println!("zone overlay '{}': {} dots", a[j + 1], extra.len());
        }
        return screenshot(
            &scene,
            cam,
            (f(5, 1280.0) as u32, f(6, 720.0) as u32),
            pl,
            extra,
            a.get(i + 1).map(String::as_str).unwrap_or("shot.png"),
        );
    }
    let player = match &saved {
        Some(p) => {
            println!(
                "save: {} at ({:.1}, {:.1}) z{}{}",
                p.name,
                p.x,
                p.y,
                p.z,
                if p.dead { " (dead)" } else { "" }
            );
            Player {
                x: p.x as f32,
                y: p.y as f32,
                z: p.z,
            }
        }
        None => spawn(&scene, gx, gy),
    };
    let mut world = World::new(1, player, &scene.collision, 40, 20);
    if let (Some(time), Some(day)) = (&saved_time, &saved_day) {
        world.clock = sim::GameClock::from_save(time, day);
        if world.clock.is_some() {
            println!("clock: experimental normal speed; Lua event dispatch enabled; weather and speed controls not connected");
        } else {
            eprintln!("save clock unavailable: unsupported date");
        }
    }
    let lua = zomboid_lua::events::EventRuntime::new()
        .map_err(|e| anyhow::anyhow!("Lua initialization: {e}"))?;
    // An existing save can provide the original persistent fishing trash seed.
    // No fish-school generation or seed updates are implied by this limited API.
    let fishing_path = if let Some(i) = a.iter().position(|s| s == "--fishing-data") {
        Some(std::path::PathBuf::from(a.get(i + 1).ok_or_else(|| {
            anyhow::anyhow!("--fishing-data requires a path")
        })?))
    } else if let Some(i) = a.iter().position(|s| s == "--save") {
        let path = std::path::Path::new(&a[i + 1]);
        let directory = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        let candidate = directory.join("fishingData.bin");
        candidate.is_file().then_some(candidate)
    } else {
        None
    };
    if let Some(path) = fishing_path {
        let saved_fishing = zomboid_lua::fishing::read_fishing_save(&path)?;
        lua.install_fishing_save(&saved_fishing)
            .map_err(|e| anyhow::anyhow!("fishing Lua API: {e}"))?;
        println!("fishing: loaded saved seeds and chum; fish abundance and other fishing APIs incomplete");
    }
    if let Some(i) = a.iter().position(|s| s == "--save") {
        let path = std::path::Path::new(&a[i + 1]);
        let directory = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        let zone_path = directory.join("map_zone.bin");
        let sand_path = directory.join("map_sand.bin");
        if zone_path.is_file() && sand_path.is_file() {
            let load = || -> anyhow::Result<zomboid_lua::fishing::FishPopulation> {
                let zones = zomboid_content::fishing_zones::parse_fishing_zones(&std::fs::read(
                    &zone_path,
                )?)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
                let abundance =
                    zomboid_content::sandbox::parse_fish_abundance(&std::fs::read(&sand_path)?)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                Ok(zomboid_lua::fishing::FishPopulation {
                    abundance,
                    zones: zones.zones,
                })
            };
            match load() {
                Ok(population) => match lua.set_fishing_population(population) {
                    Ok(()) => println!("fishing: loaded saved fish abundance and rectangle zones"),
                    Err(e) => eprintln!("fishing population unavailable: {e}"),
                },
                Err(e) => eprintln!("fishing population unavailable: {e}"),
            }
        }
    }
    if let Some(i) = a.iter().position(|s| s == "--rng-seed") {
        let value = a
            .get(i + 1)
            .ok_or_else(|| anyhow::anyhow!("--rng-seed requires eight hexadecimal digits"))?;
        if value.len() != 8 {
            anyhow::bail!("--rng-seed requires eight hexadecimal digits");
        }
        let seed = u32::from_str_radix(value, 16)?;
        let rule = zomboid_content::rng::load_rule(&root)?;
        lua.set_fishing_rng(zomboid_content::rng::CellularRng::new(
            seed.to_be_bytes(),
            rule,
        ))
        .map_err(|e| anyhow::anyhow!("fishing RNG: {e}"))?;
        println!(
            "fishing: controlled RNG seed installed; global game RNG integration is incomplete"
        );
    }
    lua.update_fishing_time(
        world
            .clock
            .as_ref()
            .and_then(|clock| clock.calendar_minutes(&chrono::Local)),
    );
    if let Some(i) = a.iter().position(|s| s == "--lua-script") {
        let path = a
            .get(i + 1)
            .ok_or_else(|| anyhow::anyhow!("--lua-script requires a path"))?;
        lua.load_file(std::path::Path::new(path))
            .map_err(|e| anyhow::anyhow!("Lua script {path}: {e}"))?;
    }
    if let Some(i) = a.iter().position(|s| s == "--headless-ticks") {
        let ticks: u32 = a
            .get(i + 1)
            .ok_or_else(|| anyhow::anyhow!("--headless-ticks requires a count"))?
            .parse()?;
        let trace_events = a.iter().any(|arg| arg == "--trace-clock-events");
        for _ in 0..ticks {
            advance_world(&mut world, &scene.collision, (0.0, 0.0), &lua)?;
            if trace_events {
                for event in &world.clock_events {
                    println!("clock event tick={} name={}", world.tick, event.name());
                }
            }
        }
        if let Some(clock) = &world.clock {
            println!(
                "clock after {ticks} ticks: bits={:08x} year={} month_index={} day_index={} nights={}",
                clock.time.to_bits(),
                clock.year,
                clock.month,
                clock.day,
                clock.nights_survived
            );
        } else {
            println!("clock after {ticks} ticks: unavailable (save time and sandbox day length required)");
        }
        return Ok(());
    }
    println!(
        "world: seed 1, {} placeholder zombies, tick {} Hz",
        world.zombies.len(),
        TICK_HZ
    );
    let el = EventLoop::new()?;
    el.set_control_flow(ControlFlow::Poll);
    el.run_app(&mut App {
        scene,
        live: None,
        cam,
        world,
        lua,
        acc: 0.0,
        keys: Default::default(),
        drag: None,
        last: std::time::Instant::now(),
    })?;
    Ok(())
}

/// Dispatch each step immediately; later steps overwrite the world's signal buffer.
fn advance_world(
    world: &mut World,
    collision: &sim::Collision,
    input: (f32, f32),
    lua: &zomboid_lua::events::EventRuntime,
) -> anyhow::Result<()> {
    world.step(collision, input);
    lua.update_fishing_time(
        world
            .clock
            .as_ref()
            .and_then(|clock| clock.calendar_minutes(&chrono::Local)),
    );
    for event in &world.clock_events {
        for error in lua
            .dispatch(event.name())
            .map_err(|e| anyhow::anyhow!("Lua event {}: {e}", event.name()))?
        {
            log::error!("Lua event {} callback: {}", error.event, error.message);
        }
    }
    Ok(())
}

#[cfg(test)]
mod event_integration_tests {
    use super::*;
    #[test]
    fn lua_chum_tracks_world_clock_and_expires() -> anyhow::Result<()> {
        let collision = sim::Collision::default();
        let mut world = World::new(
            1,
            Player {
                x: 0.5,
                y: 0.5,
                z: 0,
            },
            &collision,
            0,
            0,
        );
        world.clock = Some(sim::GameClock {
            time: 10.0,
            year: 1993,
            month: 6,
            day: 8,
            minutes_per_day: 90.0,
            effective_multiplier: 1.6,
            nights_survived: 0,
        });
        let lua = zomboid_lua::events::EventRuntime::new().map_err(|e| anyhow::anyhow!("{e}"))?;
        lua.install_fishing_save(&zomboid_lua::fishing::FishingSave {
            seed: 1,
            trash_seed: 2,
            chum: vec![],
        })
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        lua.update_fishing_time(
            world
                .clock
                .as_ref()
                .and_then(|c| c.calendar_minutes(&chrono::Local)),
        );
        lua.run_source(
            "FishSchoolManager.getInstance():addChum(100,100,200)",
            "chum",
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        world.clock.as_mut().unwrap().time = 11.0;
        advance_world(&mut world, &collision, (0.0, 0.0), &lua)?;
        assert_eq!(lua.fishing_chum_contribution(100, 100), Some(9));
        world.clock.as_mut().unwrap().time = 15.0;
        advance_world(&mut world, &collision, (0.0, 0.0), &lua)?;
        assert_eq!(lua.fishing_chum_contribution(100, 100), Some(15));
        world.clock.as_mut().unwrap().time = 15.016667;
        advance_world(&mut world, &collision, (0.0, 0.0), &lua)?;
        assert_eq!(lua.fishing_chum_contribution(100, 100), Some(0));
        Ok(())
    }

    #[test]
    fn dispatches_each_step_in_original_order() -> anyhow::Result<()> {
        let collision = sim::Collision::default();
        let mut world = World::new(
            1,
            Player {
                x: 0.5,
                y: 0.5,
                z: 0,
            },
            &collision,
            0,
            0,
        );
        world.clock = Some(sim::GameClock {
            time: 23.99998,
            year: 1993,
            month: 6,
            day: 8,
            minutes_per_day: 90.0,
            effective_multiplier: 1.6,
            nights_survived: 0,
        });
        let lua = zomboid_lua::events::EventRuntime::new().map_err(|e| anyhow::anyhow!("{e}"))?;
        lua.run_source("trace=''; for _, n in ipairs({'EveryDays','EveryHours','EveryTenMinutes','EveryOneMinute'}) do Events[n].Add(function() trace=trace..n..'|' end) end", "test").map_err(|e| anyhow::anyhow!("{e}"))?;
        advance_world(&mut world, &collision, (0.0, 0.0), &lua)?;
        advance_world(&mut world, &collision, (0.0, 0.0), &lua)?;
        let actual = lua
            .eval_string("return trace")
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let line = include_str!("../../../tests/golden/clock_events.txt")
            .lines()
            .find(|l| l.starts_with("EVENT 23.99998@0="))
            .expect("midnight oracle");
        let events = line
            .split_once('=')
            .unwrap()
            .1
            .splitn(3, ',')
            .nth(2)
            .unwrap();
        assert_eq!(actual, format!("{events}|"));
        Ok(())
    }
}

/// First unblocked square on a spiral around (gx, gy) at level 0.
fn spawn(scene: &Scene, gx: f32, gy: f32) -> Player {
    let (cx, cy) = (gx as i32, gy as i32);
    for r in 0..64i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) == r && !scene.collision.blocked(cx + dx, cy + dy, 0) {
                    return Player {
                        x: (cx + dx) as f32 + 0.5,
                        y: (cy + dy) as f32 + 0.5,
                        z: 0,
                    };
                }
            }
        }
    }
    Player { x: gx, y: gy, z: 0 }
}

/// Reads one consistent local-player row through SQLite's read-only connection.
fn load_saved_player(path: &str) -> anyhow::Result<zomboid_content::save::SavedPlayer> {
    let p = std::path::Path::new(path);
    let db = if p.is_dir() {
        p.join("players.db")
    } else {
        p.to_path_buf()
    };
    zomboid_content::save::load_player(&db)
        .map_err(anyhow::Error::msg)?
        .ok_or_else(|| anyhow::anyhow!("no local player in {}", db.display()))
}

#[cfg(test)]
mod save_tests {
    use super::load_saved_player;
    use rusqlite::Connection;

    #[test]
    fn saved_player_includes_committed_wal_without_changing_database() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let db = dir.path().join("players.db");
        let writer = Connection::open(&db)?;
        writer.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
            CREATE TABLE localPlayers(id INTEGER, name TEXT, x REAL, y REAL, z REAL, isDead INTEGER);
            INSERT INTO localPlayers VALUES(1, 'Test', 1.0, 2.0, 0.0, 0);
            PRAGMA wal_checkpoint(TRUNCATE);
            UPDATE localPlayers SET x=42.5, y=67.25, z=1.0 WHERE id=1;")?;
        let original_bytes = std::fs::read(&db)?;
        // Copying just the database loses the committed WAL update: demonstrate
        // the exact client regression with the writer still open.
        let stale = dir.path().join("main-only.db");
        std::fs::copy(&db, &stale)?;
        let old = load_saved_player(stale.to_str().ok_or_else(|| anyhow::anyhow!("path"))?)?;
        assert_eq!((old.x, old.y, old.z), (1.0, 2.0, 0));
        let current =
            load_saved_player(dir.path().to_str().ok_or_else(|| anyhow::anyhow!("path"))?)?;
        assert_eq!((current.x, current.y, current.z), (42.5, 67.25, 1));
        assert_eq!(std::fs::read(&db)?, original_bytes);
        let x: f64 = writer.query_row("SELECT x FROM localPlayers WHERE id=1", [], |row| {
            row.get(0)
        })?;
        assert_eq!(x, 42.5);
        Ok(())
    }
}
