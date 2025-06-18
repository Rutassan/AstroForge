use glam::{Mat4, Vec3};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
use wgpu::util::DeviceExt;

pub struct Renderer {
    pub surface: wgpu::Surface,
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
}

impl Renderer {
    pub async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = unsafe { instance.create_surface(window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("device");

        let surface_caps = surface.get_capabilities(&adapter);
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
        surface.configure(&device, &config);

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
        let (artifact_vertex, artifact_index, artifact_indices) =
            create_artifact_buffers(&device);

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
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
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

    pub fn render(&mut self) {
        let output = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .expect("failed to acquire next surface texture")
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
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
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.camera_bind, &[]);
            rpass.set_bind_group(1, &self.default_bind, &[]);
            rpass.set_vertex_buffer(0, self.floor_vertex.slice(..));
            rpass.set_index_buffer(self.floor_index.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..self.floor_indices, 0, 0..1);

            rpass.set_bind_group(1, &self.artifact_bind, &[]);
            rpass.set_vertex_buffer(0, self.artifact_vertex.slice(..));
            rpass.set_index_buffer(self.artifact_index.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..self.artifact_indices, 0, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        output.present();
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
        Vertex { position: [-0.5, 0.0, 0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [0.5, 0.0, 0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [0.5, 1.0, 0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [-0.5, 1.0, 0.5], color: [1.0, 1.0, 1.0] },
        // back
        Vertex { position: [-0.5, 0.0, -0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [0.5, 0.0, -0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [0.5, 1.0, -0.5], color: [1.0, 1.0, 1.0] },
        Vertex { position: [-0.5, 1.0, -0.5], color: [1.0, 1.0, 1.0] },
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
