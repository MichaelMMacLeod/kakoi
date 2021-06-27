use crate::arena::{ArenaKey, Structure, Value};
use crate::spatial_bound::SpatialBound;
use crate::{camera::Camera, spatial_tree::SpatialTreeData, sphere::Sphere};
use slotmap::SlotMap;
use std::collections::HashMap;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    fn new(view_projection_matrix: cgmath::Matrix4<f32>) -> Self {
        Self {
            view_proj: view_projection_matrix.into(),
        }
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

struct BoundTextureInstances {
    diffuse_bind_group: wgpu::BindGroup,
    raw_texture_instances: Vec<RawTextureInstance>,
    buffer_cache: Option<wgpu::Buffer>,
}

struct TextureInstances {
    instances: Vec<TextureInstance>,
}

impl BoundTextureInstances {
    fn instantiate_buffer_cache<'a, 'b>(
        buffer_cache: &'b mut Option<wgpu::Buffer>,
        instances: &'b Vec<RawTextureInstance>,
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
    bound: HashMap<ArenaKey, BoundTextureInstances>,
    unbound: HashMap<ArenaKey, TextureInstances>,
    vertex_buffer_data: Vec<Vertex>,
    vertex_buffer: wgpu::Buffer,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    uniform_buffer_stale: bool,
    uniform_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
}

impl ImageRenderer {
    pub fn new<'a>(device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) -> Self {
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

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image renderer uniform buffer"),
            size: std::mem::size_of::<Uniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

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

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("image renderer uniform bind group"),
        });

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
                buffers: &[Vertex::desc(), RawTextureInstance::desc()],
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
            bound: HashMap::new(),
            unbound: HashMap::new(),
            vertex_buffer_data,
            vertex_buffer,
            texture_bind_group_layout,
            uniform_buffer_stale: true,
            uniform_buffer,
            uniform_bind_group,
            render_pipeline,
        }
    }

    pub fn with_image<'a>(&mut self, spatial_tree_data: SpatialTreeData) {
        self.unbound
            .entry(spatial_tree_data.key)
            .or_insert(TextureInstances {
                instances: Vec::with_capacity(1),
            })
            .instances
            .push(TextureInstance {
                sphere: spatial_tree_data.bounds,
            });
    }

    pub fn resize(&mut self) {
        self.uniform_buffer_stale = true;
    }

    pub fn render<'a>(
        &mut self,
        device: &'a wgpu::Device,
        queue: &'a mut wgpu::Queue,
        command_encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
        camera: &'a mut Camera,
        store: &'a SlotMap<ArenaKey, Value>,
    ) {
        if self.uniform_buffer_stale {
            queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::cast_slice(&[Uniforms::new(*camera.view_projection_matrix())]),
            );
            self.uniform_buffer_stale = false;
        }

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

        let texture_bind_group_layout = &self.texture_bind_group_layout;

        for (image_key, mut unbound_image_instance) in self.unbound.drain() {
            let image = match &store.get(image_key).unwrap().structure {
                Structure::Image(i) => i,
                _ => panic!(),
            };
            let dimensions = image.dimensions();
            let aspect_ratio = dimensions.0 as f32 / dimensions.1 as f32;

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
                image.as_ref(),
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
            self.bound
                .entry(image_key)
                .or_insert_with(|| BoundTextureInstances {
                    diffuse_bind_group,
                    buffer_cache: None,
                    raw_texture_instances: Vec::with_capacity(
                        unbound_image_instance.instances.len(),
                    ),
                })
                .raw_texture_instances
                .append(
                    &mut unbound_image_instance
                        .instances
                        .drain(..)
                        .map(|i| i.to_raw(aspect_ratio))
                        .collect(),
                );
        }

        for (_, bound_texture_instances) in &mut self.bound {
            let BoundTextureInstances {
                diffuse_bind_group,
                buffer_cache,
                raw_texture_instances: instances,
            } = bound_texture_instances;
            if instances.len() > 0 {
                render_pass.set_bind_group(0, diffuse_bind_group, &[]);
                render_pass.set_vertex_buffer(
                    1,
                    BoundTextureInstances::instantiate_buffer_cache(
                        buffer_cache,
                        instances,
                        device,
                    )
                    .slice(..),
                );
                render_pass.draw(
                    0..self.vertex_buffer_data.len() as _,
                    0..instances.len() as _,
                );
            }
        }
    }

    pub fn invalidate(&mut self) {
        self.unbound.clear();
        for (_, bound_texture_instance) in &mut self.bound {
            bound_texture_instance.buffer_cache = None;
            bound_texture_instance.raw_texture_instances.clear();
        }
    }
}

struct TextureInstance {
    sphere: SpatialBound,
}

impl TextureInstance {
    fn to_raw(&self, aspect_ratio: f32) -> RawTextureInstance {
        RawTextureInstance::new(&self.sphere, aspect_ratio)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawTextureInstance {
    model: [[f32; 4]; 4],
}

impl RawTextureInstance {
    pub fn new(bound: &SpatialBound, aspect_ratio: f32) -> Self {
        match bound {
            SpatialBound::Sphere(sphere) => {
                let (scale_x, scale_y) = sphere.as_rectangle_bounds(aspect_ratio);
                let scale = cgmath::Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);
                let translation = cgmath::Matrix4::from_translation(sphere.center);
                Self {
                    model: (translation * scale).into(),
                }
            }
            SpatialBound::SquareCuboid(square_cuboid) => {
                let (width, height) = square_cuboid.dimensions_2d();
                
            }
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
