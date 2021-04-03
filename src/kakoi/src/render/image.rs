use std::collections::HashMap;

use futures::sink::Unfold;
use wgpu::{util::DeviceExt, TextureView};

use crate::{sphere::Sphere, store};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj<'a>(&mut self, view_projection_matrix: cgmath::Matrix4<f32>) {
        self.view_proj = view_projection_matrix.into();
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    texture_position: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
            ],
        }
    }

    fn rectangle(width: f32, height: f32) -> Vec<Self> {
        let half_width = width * 0.5;
        let half_height = height * 0.5;
        //  1-----0,3
        //  |      |
        //  |      |
        // 2,4-----5
        let v0 = Self {
            position: [half_width, half_height, 0.0],
            texture_position: [1.0, 0.0],
        };
        let v1 = Self {
            position: [-half_width, half_height, 0.0],
            texture_position: [0.0, 0.0],
        };
        let v2 = Self {
            position: [-half_width, -half_height, 0.0],
            texture_position: [0.0, 1.0],
        };
        let v3 = v0;
        let v4 = v2;
        let v5 = Self {
            position: [half_width, -half_height, 0.0],
            texture_position: [1.0, 1.0],
        };
        vec![v0, v1, v2, v3, v4, v5]
    }

    fn square() -> Vec<Self> {
        Self::rectangle(1.0, 1.0)
    }
}

struct TextureInstances {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    diffuse_bind_group: wgpu::BindGroup,
    instances: Vec<ImageInstance>,
    buffer_cache: Option<wgpu::Buffer>,
}

impl TextureInstances {
    fn instantiate_buffer_cache<'a, 'b>(
        buffer_cache: &'b mut Option<wgpu::Buffer>,
        instances: &'b Vec<ImageInstance>,
        device: &'a wgpu::Device,
    ) -> &'b wgpu::Buffer {
        if buffer_cache.is_none() {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("image renderer instance buffer"),
                contents: bytemuck::cast_slice(instances),
                usage: wgpu::BufferUsage::VERTEX,
            });

            *buffer_cache = Some(buffer);
        }

        buffer_cache.as_ref().unwrap()
    }
}

pub struct ImageRenderer {
    images: HashMap<store::Key, TextureInstances>,
    vertex_buffer_data: Vec<Vertex>,
    vertex_buffer: wgpu::Buffer,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
}

impl ImageRenderer {
    fn build_uniform_bind_group<'a>(
        device: &'a wgpu::Device,
        uniform_bind_group_layout: &'a wgpu::BindGroupLayout,
        uniform_buffer: &'a wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("image renderer uniform bind group"),
        })
    }

    fn build_uniform_buffer<'a>(device: &'a wgpu::Device, uniforms: Uniforms) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("image renderer uniform buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        })
    }

    pub fn new<'a>(
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    ) -> Self {
        let vertex_buffer_data = Vertex::square();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("image renderer vertex buffer"),
            contents: bytemuck::cast_slice(&vertex_buffer_data),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("../shaders/build/image.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("../shaders/build/image.frag.spv"));

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj((*view_projection_matrix).into());

        let uniform_buffer = Self::build_uniform_buffer(device, uniforms);

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("image renderer uniform bind group layout"),
            });

        let uniform_bind_group =
            Self::build_uniform_bind_group(device, &uniform_bind_group_layout, &uniform_buffer);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },

                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
                label: Some("image renderer texture bind group layout"),
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("image renderer pipeline layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image renderer pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Vertex::desc(), ImageInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    alpha_blend: wgpu::BlendState::REPLACE,
                    color_blend: wgpu::BlendState::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        Self {
            images: HashMap::new(),
            vertex_buffer_data,
            vertex_buffer,
            texture_bind_group_layout,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            uniform_bind_group_layout,
            render_pipeline,
        }
    }

    pub fn with_image<'a>(
        &mut self,
        device: &'a wgpu::Device,
        queue: &'a mut wgpu::Queue,
        store: &'a store::Store,
        sphere: Sphere,
        key: store::Key,
    ) {
        let image = store.get(&key).unwrap().image().unwrap();
        let dimensions = image.dimensions();
        let texture_bind_group_layout = &self.texture_bind_group_layout;

        self.images
            .entry(key)
            .or_insert_with(|| {
                eprintln!("Uploading image data");
                let size = {
                    wgpu::Extent3d {
                        width: dimensions.0,
                        height: dimensions.1,
                        depth: 1,
                    }
                };

                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
                });

                queue.write_texture(
                    wgpu::TextureCopyView {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                    },
                    image,
                    wgpu::TextureDataLayout {
                        offset: 0,
                        bytes_per_row: 4 * dimensions.0,
                        rows_per_image: dimensions.1,
                    },
                    size,
                );

                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Nearest,
                    mipmap_filter: wgpu::FilterMode::Nearest,
                    ..Default::default()
                });

                let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                    label: Some("image renderer diffuse bind group"),
                });

                TextureInstances {
                    texture,
                    view,
                    sampler,
                    diffuse_bind_group,
                    instances: Vec::with_capacity(1),
                    buffer_cache: None,
                }
            })
            .instances
            .push(ImageInstance::new(
                &sphere,
                dimensions.0 as f32 / dimensions.1 as f32,
            ));

        eprintln!("self.images.len() = {}", self.images.len());
    }

    pub fn resize(&mut self, device: &wgpu::Device, view_projection_matrix: &cgmath::Matrix4<f32>) {
        self.uniforms
            .update_view_proj((*view_projection_matrix).into());
        self.uniform_buffer = Self::build_uniform_buffer(device, self.uniforms);
        self.uniform_bind_group =
            Self::build_uniform_bind_group(device, &self.uniform_bind_group_layout, &self.uniform_buffer);
    }

    pub fn render<'a>(
        &mut self,
        device: &'a wgpu::Device,
        command_encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
    ) {
        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("image renderer render pass"),
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        for (_, texture_instances) in &mut self.images {
            let num_instances = texture_instances.instances.len();
            if num_instances > 0 {
                render_pass.set_bind_group(0, &texture_instances.diffuse_bind_group, &[]);
                render_pass.set_vertex_buffer(
                    1,
                    TextureInstances::instantiate_buffer_cache(
                        &mut texture_instances.buffer_cache,
                        &texture_instances.instances,
                        device,
                    )
                    .slice(..),
                );
                render_pass.draw(
                    0..self.vertex_buffer_data.len() as _,
                    0..num_instances as _,
                );
            }
        }
    }

    pub fn invalidate(&mut self) {
        for (_, texture_instance) in &mut self.images {
            texture_instance.instances.clear();
            texture_instance.buffer_cache = None;
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageInstance {
    model: [[f32; 4]; 4],
}

impl ImageInstance {
    pub fn new(sphere: &Sphere, aspect_ratio: f32) -> Self {
        let scale = cgmath::Matrix4::from_nonuniform_scale(aspect_ratio, 1.0 / aspect_ratio, 1.0);
        let scale = cgmath::Matrix4::from_scale(sphere.radius) * scale;
        let translation = cgmath::Matrix4::from_translation(sphere.center);
        Self {
            model: (translation * scale).into(),
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}
