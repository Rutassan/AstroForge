use glam::{Mat4, Vec3};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use wgpu::util::DeviceExt;
use wgpu_glyph::GlyphBrush as WgpuGlyphBrush;
use wgpu_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, Section, Text};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Renderer {
    pub surface: Option<wgpu::Surface>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub camera_bind: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub floor_vertex: wgpu::Buffer,
    pub floor_index: wgpu::Buffer,
    pub floor_indices: u32,
    pub artifact_vertex: wgpu::Buffer,
    pub artifact_index: wgpu::Buffer,
    pub artifact_indices: u32,
    pub default_bind: wgpu::BindGroup,
    pub artifact_bind: wgpu::BindGroup,
    artifact_buffer: wgpu::Buffer,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub glyph_brush: WgpuGlyphBrush<()>,
    pub offscreen_texture: Option<wgpu::Texture>,
    pub offscreen_view: Option<wgpu::TextureView>,
}

#[derive(Clone, Copy)]
pub struct CubeInstance {
    pub position: Vec3,
    pub size: f32,
    pub color: [f32; 3],
}

impl Renderer {
    pub async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = Some(unsafe { instance.create_surface(window) }.unwrap());
        let adapter = wgpu::util::initialize_adapter_from_env_or_default(
            &instance,
            surface.as_ref(),
        )
        .await
        .expect("No adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("device");

        let surface_caps = surface.as_ref().unwrap().get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            // Avoid creating an empty slice which may trip debug assertions in
            // wgpu by providing the surface format here as well.
            view_formats: vec![surface_format],
        };
        surface.as_ref().unwrap().configure(&device, &config);

        let (depth_texture, depth_view) = create_depth_texture(&device, &config, "depth texture");

        // camera uniform
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CameraUniform {
            view_proj: [[f32; 4]; 4],
        }

        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bind layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera bind group"),
        });

        // artifact intensity uniform
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ArtifactUniform {
            intensity: f32,
        }
        let artifact_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Artifact Buffer"),
            contents: bytemuck::bytes_of(&ArtifactUniform { intensity: 0.2 }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let default_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Default Artifact Buffer"),
            contents: bytemuck::bytes_of(&ArtifactUniform { intensity: 1.0 }),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let artifact_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("artifact bind layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let default_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &artifact_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: default_buffer.as_entire_binding(),
            }],
            label: Some("default artifact bind group"),
        });
        let artifact_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &artifact_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: artifact_buffer.as_entire_binding(),
            }],
            label: Some("artifact bind group"),
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("../../assets/unlit.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &artifact_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let (vertex_buffer, index_buffer, num_indices) = create_cube_buffers(&device);
        let (floor_vertex, floor_index, floor_indices) = create_floor_buffers(&device);
        let (artifact_vertex, artifact_index, artifact_indices) = create_artifact_buffers(&device);

        // Offscreen texture
        let offscreen_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let offscreen_view = offscreen_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Glyph brush
        let font_path = "assets/DejaVuSans.ttf";
        let mut font_valid = false;
        if Path::new(font_path).exists() {
            if let Ok(data) = fs::read(font_path) {
                if data.len() > 4
                    && (data[0..4] == [0x00, 0x01, 0x00, 0x00]
                        || data[0..4] == [0x4F, 0x54, 0x54, 0x4F])
                {
                    font_valid = true;
                }
            }
        }
        if !font_valid {
            let urls = [
                "https://github.com/dejavu-fonts/dejavu-fonts/raw/master/ttf/DejaVuSans.ttf",
                "https://downloads.sourceforge.net/project/dejavu/dejavu/2.37/dejavu-fonts-ttf-2.37.zip"
            ];
            let mut downloaded = false;
            for url in urls.iter() {
                println!("[INFO] Пытаюсь скачать шрифт: {url}");
                let resp = reqwest::blocking::get(*url);
                if let Ok(resp) = resp {
                    if resp.status().is_success() {
                        let bytes = resp.bytes();
                        if let Ok(bytes) = bytes {
                            // Если это zip-архив, извлечь ttf
                            if url.ends_with(".zip") {
                                if let Ok(mut archive) =
                                    zip::ZipArchive::new(std::io::Cursor::new(&bytes))
                                {
                                    for i in 0..archive.len() {
                                        let mut file = archive.by_index(i).unwrap();
                                        if file.name().ends_with("DejaVuSans.ttf") {
                                            let mut ttf_bytes = Vec::new();
                                            use std::io::Read;
                                            file.read_to_end(&mut ttf_bytes).unwrap();
                                            fs::create_dir_all("assets").ok();
                                            if fs::write(font_path, &ttf_bytes).is_ok() {
                                                downloaded = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                            } else {
                                fs::create_dir_all("assets").ok();
                                if fs::write(font_path, &bytes).is_ok() {
                                    downloaded = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if !downloaded {
                eprintln!("[ERROR] Не удалось скачать или извлечь шрифт DejaVuSans.ttf ни с одного источника");
                std::process::exit(1);
            }
        }
        // Диагностика: размер и первые байты файла шрифта
        match fs::read(font_path) {
            Ok(data) => {
                println!("[DEBUG] Размер DejaVuSans.ttf: {} байт", data.len());
                let preview: Vec<String> =
                    data.iter().take(16).map(|b| format!("{:02X}", b)).collect();
                println!("[DEBUG] Первые 16 байт: {}", preview.join(" "));
            }
            Err(e) => {
                eprintln!("[ERROR] Не удалось прочитать файл шрифта для диагностики: {e}");
            }
        }
        let font =
            match ab_glyph::FontArc::try_from_vec(fs::read(font_path).expect("read font file")) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[ERROR] Не удалось загрузить TTF-шрифт: {e}");
                    std::process::exit(1);
                }
            };
        let glyph_brush = GlyphBrushBuilder::using_font(font).build(&device, surface_format);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            camera_bind,
            camera_buffer,
            pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            floor_vertex,
            floor_index,
            floor_indices,
            artifact_vertex,
            artifact_index,
            artifact_indices,
            default_bind,
            artifact_bind,
            artifact_buffer,
            depth_texture,
            depth_view,
            glyph_brush,
            offscreen_texture: None,
            offscreen_view: None,
        }
    }

    pub async fn new_headless(width: u32, height: u32) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = wgpu::util::initialize_adapter_from_env_or_default(
            &instance,
            None,
        )
        .await
        .expect("No adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("device");
        let texture_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![texture_format],
        };
        let (depth_texture, depth_view) = create_depth_texture(&device, &config, "depth texture");

        // camera uniform
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CameraUniform {
            view_proj: [[f32; 4]; 4],
        }

        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bind layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera bind group"),
        });
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ArtifactUniform {
            intensity: f32,
        }
        let artifact_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Artifact Buffer"),
            contents: bytemuck::bytes_of(&ArtifactUniform { intensity: 0.2 }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let default_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Default Artifact Buffer"),
            contents: bytemuck::bytes_of(&ArtifactUniform { intensity: 1.0 }),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let artifact_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("artifact bind layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let default_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &artifact_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: default_buffer.as_entire_binding(),
            }],
            label: Some("default artifact bind group"),
        });
        let artifact_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &artifact_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: artifact_buffer.as_entire_binding(),
            }],
            label: Some("artifact bind group"),
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("../../assets/unlit.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &artifact_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let (vertex_buffer, index_buffer, num_indices) = create_cube_buffers(&device);
        let (floor_vertex, floor_index, floor_indices) = create_floor_buffers(&device);
        let (artifact_vertex, artifact_index, artifact_indices) = create_artifact_buffers(&device);

        // Offscreen texture
        let offscreen_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let offscreen_view = offscreen_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Glyph brush
        let font_path = "assets/DejaVuSans.ttf";
        let font =
            ab_glyph::FontArc::try_from_vec(fs::read(font_path).expect("read font file")).unwrap();
        let glyph_brush = GlyphBrushBuilder::using_font(font).build(&device, texture_format);
        Self {
            surface: None,
            device,
            queue,
            config,
            size: winit::dpi::PhysicalSize::new(width, height),
            camera_bind,
            camera_buffer,
            pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            floor_vertex,
            floor_index,
            floor_indices,
            artifact_vertex,
            artifact_index,
            artifact_indices,
            default_bind,
            artifact_bind,
            artifact_buffer,
            depth_texture,
            depth_view,
            glyph_brush,
            offscreen_texture: Some(offscreen_texture),
            offscreen_view: Some(offscreen_view),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            if let Some(surface) = &self.surface {
                surface.configure(&self.device, &self.config);
            }
            let (tex, view) = create_depth_texture(&self.device, &self.config, "depth texture");
            self.depth_texture = tex;
            self.depth_view = view;
        }
    }

    pub fn update_camera(&self, view_proj: &Mat4) {
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CameraUniform {
            view_proj: [[f32; 4]; 4],
        }
        let data = CameraUniform {
            view_proj: (*view_proj).to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&data));
    }

    pub fn update_artifact(&self, intensity: f32) {
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ArtifactUniform {
            intensity: f32,
        }
        let data = ArtifactUniform { intensity };
        self.queue
            .write_buffer(&self.artifact_buffer, 0, bytemuck::bytes_of(&data));
    }

    pub fn render_overlay_text(
        &mut self,
        text: &str,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        staging_belt: &mut wgpu::util::StagingBelt,
    ) {
        let section = Section {
            screen_position: (30.0, 30.0),
            bounds: (
                self.size.width as f32 - 60.0,
                self.size.height as f32 - 60.0,
            ),
            text: vec![Text::new(text)
                .with_color([1.0, 1.0, 0.5, 1.0])
                .with_scale(36.0)],
            ..Section::default()
        };
        self.glyph_brush.queue(section);
        self.glyph_brush
            .draw_queued(
                &self.device,
                staging_belt,
                encoder,
                view,
                self.size.width,
                self.size.height,
            )
            .expect("Draw glyphs");
    }

    pub fn render_health_text(
        &mut self,
        health: i32,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        staging_belt: &mut wgpu::util::StagingBelt,
    ) {
        let text = format!("Health: {}", health);
        let section = Section {
            screen_position: (30.0, 70.0),
            bounds: (
                self.size.width as f32 - 60.0,
                self.size.height as f32 - 60.0,
            ),
            text: vec![Text::new(&text)
                .with_color([0.0, 1.0, 0.0, 1.0])
                .with_scale(28.0)],
            ..Section::default()
        };
        self.glyph_brush.queue(section);
        self.glyph_brush
            .draw_queued(
                &self.device,
                staging_belt,
                encoder,
                view,
                self.size.width,
                self.size.height,
            )
            .expect("Draw glyphs");
    }

    pub fn render(&mut self, overlay_text: Option<&str>, health: i32, cubes: &[CubeInstance]) {
        use wgpu::util::StagingBelt;
        let mut staging_belt = StagingBelt::new(1024);
        if let Some(surface) = &self.surface {
            let output = match surface.get_current_texture() {
                Ok(frame) => frame,
                Err(_) => {
                    self.surface
                        .as_ref()
                        .unwrap()
                        .configure(&self.device, &self.config);
                    self.surface
                        .as_ref()
                        .unwrap()
                        .get_current_texture()
                        .unwrap()
                }
            };
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                });
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, &self.camera_bind, &[]);
                render_pass.set_bind_group(1, &self.default_bind, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
                // ...добавьте рендер кубов, артефактов и т.д. по вашей логике...
            }
            if let Some(text) = overlay_text {
                self.render_overlay_text(text, &mut encoder, &view, &mut staging_belt);
            }
            self.render_health_text(health, &mut encoder, &view, &mut staging_belt);
            staging_belt.finish();
            self.queue.submit(Some(encoder.finish()));
            output.present();
        } else {
            // Headless/offscreen: рендерим в offscreen_view
            let mut view = self.offscreen_view.take().unwrap();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder (Headless)"),
                });
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                });
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, &self.camera_bind, &[]);
                render_pass.set_bind_group(1, &self.default_bind, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
                // ...добавьте рендер кубов, артефактов и т.д. по вашей логике...
            }
            if let Some(text) = overlay_text {
                self.render_overlay_text(text, &mut encoder, &view, &mut staging_belt);
            }
            self.render_health_text(health, &mut encoder, &view, &mut staging_belt);
            staging_belt.finish();
            self.queue.submit(Some(encoder.finish()));
            self.device.poll(wgpu::Maintain::Wait);
            self.offscreen_view = Some(view);
        }
    }

    pub fn get_frame_rgba8(&self) -> Vec<u8> {
        let width = self.size.width;
        let height = self.size.height;
        let buffer_size = (width * height * 4) as wgpu::BufferAddress;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screenshot Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let src_texture = if let Some(surface) = &self.surface {
            &surface.get_current_texture().unwrap().texture
        } else {
            self.offscreen_texture.as_ref().unwrap()
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Screenshot Encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: src_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);
        // map_async через callback и condvar
        let slice = buffer.slice(..);
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = pair.clone();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let (lock, cvar) = &*pair2;
            let mut done = lock.lock().unwrap();
            *done = true;
            cvar.notify_one();
        });
        // Ждём завершения map_async
        let (lock, cvar) = &*pair;
        let mut done = lock.lock().unwrap();
        while !*done {
            done = cvar.wait(done).unwrap();
        }
        let data = slice.get_mapped_range().to_vec();
        drop(slice);
        buffer.unmap();
        data
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

fn create_cube_buffers(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let vertices = [
        // front
        Vertex {
            position: [-0.5, 0.0, 0.5],
            color: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [0.5, 0.0, 0.5],
            color: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [0.5, 1.0, 0.5],
            color: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [-0.5, 1.0, 0.5],
            color: [1.0, 1.0, 1.0],
        },
        // back
        Vertex {
            position: [-0.5, 0.0, -0.5],
            color: [1.0, 0.0, 0.0],
        },
        Vertex {
            position: [0.5, 0.0, -0.5],
            color: [0.0, 1.0, 0.0],
        },
        Vertex {
            position: [0.5, 1.0, -0.5],
            color: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [-0.5, 1.0, -0.5],
            color: [1.0, 1.0, 1.0],
        },
    ];
    let indices: &[u16] = &[
        0, 1, 2, 2, 3, 0, // front
        1, 5, 6, 6, 2, 1, // right
        5, 4, 7, 7, 6, 5, // back
        4, 0, 3, 3, 7, 4, // left
        3, 2, 6, 6, 7, 3, // top
        4, 5, 1, 1, 0, 4, // bottom
    ];
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cube Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cube Index Buffer"),
        contents: bytemuck::cast_slice(indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    (vertex_buffer, index_buffer, indices.len() as u32)
}

fn create_floor_buffers(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let size = 20.0f32;
    let y = 0.0f32;
    let vertices = [
        Vertex {
            position: [-size, y, -size],
            color: [0.3, 0.3, 0.3],
        },
        Vertex {
            position: [size, y, -size],
            color: [0.3, 0.3, 0.3],
        },
        Vertex {
            position: [size, y, size],
            color: [0.3, 0.3, 0.3],
        },
        Vertex {
            position: [-size, y, size],
            color: [0.3, 0.3, 0.3],
        },
    ];
    // WGPU expects counter-clockwise winding for front faces. Arrange the
    // floor indices accordingly so the surface is visible from above.
    let indices: &[u16] = &[0, 2, 1, 0, 3, 2];
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Floor Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Floor Index Buffer"),
        contents: bytemuck::cast_slice(indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    (vertex_buffer, index_buffer, indices.len() as u32)
}

fn create_artifact_buffers(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let base_vertices = [
        // front
        Vertex {
            position: [-0.5, 0.0, 0.5],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [0.5, 0.0, 0.5],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [0.5, 1.0, 0.5],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [-0.5, 1.0, 0.5],
            color: [1.0, 1.0, 1.0],
        },
        // back
        Vertex {
            position: [-0.5, 0.0, -0.5],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [0.5, 0.0, -0.5],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [0.5, 1.0, -0.5],
            color: [1.0, 1.0, 1.0],
        },
        Vertex {
            position: [-0.5, 1.0, -0.5],
            color: [1.0, 1.0, 1.0],
        },
    ];
    let base_indices: &[u16] = &[
        0, 1, 2, 2, 3, 0, // front
        1, 5, 6, 6, 2, 1, // right
        5, 4, 7, 7, 6, 5, // back
        4, 0, 3, 3, 7, 4, // left
        3, 2, 6, 6, 7, 3, // top
        4, 5, 1, 1, 0, 4, // bottom
    ];

    let count = 28u16;
    let radius = 3.0f32;
    let mut vertices = Vec::with_capacity((base_vertices.len() as u16 * count) as usize);
    let mut indices = Vec::with_capacity((base_indices.len() as u16 * count) as usize);

    for i in 0..count {
        let angle = i as f32 / count as f32 * std::f32::consts::TAU;
        let x = radius * angle.cos();
        let z = radius * angle.sin();
        let base = i * base_vertices.len() as u16;
        for v in &base_vertices {
            vertices.push(Vertex {
                position: [v.position[0] + x, v.position[1], v.position[2] + z],
                color: v.color,
            });
        }
        for idx in base_indices {
            indices.push(base + *idx);
        }
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Artifact Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Artifact Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    (vertex_buffer, index_buffer, indices.len() as u32)
}

fn create_depth_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    label: &str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let size = wgpu::Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let desc = wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}
