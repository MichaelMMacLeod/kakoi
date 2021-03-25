use crate::camera::Camera;
use crate::sampling_config::SamplingConfig;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use wgpu_glyph::GlyphCruncher;

#[derive(Clone, Copy)]
pub struct Sphere {
    pub center: cgmath::Vector3<f32>,
    pub radius: f32,
}

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

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

pub const MIN_RADIUS: f32 = 0.98;
pub const MAX_RADIUS: f32 = 1.0;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float3,
            }],
        }
    }

    fn make_circle(steps: u32, min_radius: f32, max_radius: f32) -> Vec<Vertex> {
        let mut theta = 0.0;
        let mut result = Vec::new();
        let step = 2.0 * std::f32::consts::PI / steps as f32;
        while theta < 2.0 * std::f32::consts::PI {
            let (x1, y1) = (theta.cos(), theta.sin());
            let (x2, y2) = ((theta + step).cos(), (theta + step).sin());
            let v1 = Vertex {
                position: [x1 * min_radius, y1 * min_radius, 0.0],
            };
            let v2 = Vertex {
                position: [x2 * min_radius, y2 * min_radius, 0.0],
            };
            let v3 = Vertex {
                position: [x1 * max_radius, y1 * max_radius, 0.0],
            };
            let v4 = Vertex {
                position: [x1 * max_radius, y1 * max_radius, 0.0],
            };
            let v5 = Vertex {
                position: [x2 * min_radius, y2 * min_radius, 0.0],
            };
            let v6 = Vertex {
                position: [x2 * max_radius, y2 * max_radius, 0.0],
            };
            result.append(&mut [v1, v2, v3, v4, v5, v6].into());
            theta += step;
        }
        result
    }

    fn circle() -> Vec<Vertex> {
        Self::make_circle(200, MIN_RADIUS, MAX_RADIUS)
    }
}

pub struct CircleConstraintBuilder {
    constraints: HashMap<NodeIndex<u32>, Vec<Sphere>>,
    num_instances: u32,
    instances_cache: Option<wgpu::Buffer>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    vertex_buffer_data: Vec<Vertex>,
    sampling_config: SamplingConfig,
}

impl CircleConstraintBuilder {
    pub fn new<'a>(device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) -> Self {
        let vertex_buffer_data = Vertex::circle();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("CircleConstraint vertex buffer"),
            contents: bytemuck::cast_slice(&vertex_buffer_data),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/build/shader.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/build/shader.frag.spv"));

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&Camera::new(sc_desc.width as f32 / sc_desc.height as f32));

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
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
                label: Some("uniform_bind_group_layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let sample_count = 4;
        let multisampled_framebuffer =
            Self::create_mutisampled_framebuffer(&device, &sc_desc, sample_count);
        let sampling_config = SamplingConfig::Multi {
            sample_count,
            multisampled_framebuffer,
        };

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Vertex::desc(), CircleConstraintInstance::desc()],
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
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        Self {
            constraints: HashMap::new(),
            num_instances: 0,
            instances_cache: None,
            render_pipeline,
            vertex_buffer,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            vertex_buffer_data,
            sampling_config,
        }
    }

    pub fn with_constraint(&mut self, node_index: NodeIndex<u32>, sphere: Sphere) {
        self.constraints
            .entry(node_index)
            .or_insert_with(|| Vec::with_capacity(1))
            .push(sphere);
        self.num_instances += 1;
    }

    pub fn build_instances<'a, 'b>(
        instances_cache: &'b mut Option<wgpu::Buffer>,
        constraints: &'b HashMap<NodeIndex<u32>, Vec<Sphere>>,
        device: &'a wgpu::Device,
    ) -> &'b wgpu::Buffer {
        if instances_cache.is_none() {
            let mut instances: Vec<CircleConstraintInstance> = Vec::new();

            let mut build_onekey_instances = |spheres| {
                for sphere in spheres {
                    instances.push(CircleConstraintInstance::new(sphere));
                }
            };

            for (_, spheres) in constraints {
                build_onekey_instances(spheres);
            }

            *instances_cache = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("CircleConstraintInstance instance buffer"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsage::VERTEX,
                }),
            );
        }

        instances_cache.as_ref().unwrap()
    }

    pub fn update<'a>(&mut self, queue: &'a mut wgpu::Queue, camera: &'a Camera) {
        self.uniforms.update_view_proj(camera);
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    pub fn resize<'a>(&mut self, device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) {
        self.sampling_config = match self.sampling_config {
            SamplingConfig::Single => SamplingConfig::Single,
            SamplingConfig::Multi { sample_count, .. } => SamplingConfig::Multi {
                sample_count,
                multisampled_framebuffer: Self::create_mutisampled_framebuffer(
                    device,
                    sc_desc,
                    sample_count,
                ),
            },
        };
    }

    pub fn render<'a>(
        &mut self,
        device: &'a wgpu::Device,
        command_encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
        sc_desc: &'a wgpu::SwapChainDescriptor,
    ) {
        // This is supposed to be rgb(33,33,33,256), but it ends up being a bit too dark on screen.
        // I don't know why---if you have more knowledge of color spaces, please help!
        //
        // get_swap_chain_preferred_format: https://docs.rs/wgpu/0.7.0/wgpu/struct.Adapter.html#method.get_swap_chain_preferred_format
        //   - This returns Bgra8UnormSrgb on my computer.
        // sRGB color space: https://en.wikipedia.org/wiki/SRGB
        let grayish_color = (33.0f64 / 256.0f64).powf(2.2f64);

        let color_attachment_descriptor = match &self.sampling_config {
            SamplingConfig::Single => wgpu::RenderPassColorAttachmentDescriptor {
                attachment: texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: grayish_color,
                        g: grayish_color,
                        b: grayish_color,
                        a: 1.0,
                    }),
                    store: true,
                },
            },
            SamplingConfig::Multi {
                sample_count: _sample_count,
                multisampled_framebuffer,
            } => wgpu::RenderPassColorAttachmentDescriptor {
                attachment: multisampled_framebuffer,
                resolve_target: Some(texture_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: grayish_color,
                        g: grayish_color,
                        b: grayish_color,
                        a: 1.0,
                    }),
                    store: true,
                },
            },
        };

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[color_attachment_descriptor],
            depth_stencil_attachment: None,
        });

        let instance_buffer =
            Self::build_instances(&mut self.instances_cache, &self.constraints, device);

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        let width = sc_desc.width as f32;
        let height = sc_desc.height as f32;
        render_pass.set_viewport(0.0, 0.0, width, height, 0.0, 1.0);
        render_pass.draw(0..self.vertex_buffer_data.len() as _, 0..self.num_instances);
    }

    fn create_mutisampled_framebuffer(
        device: &wgpu::Device,
        sc_desc: &wgpu::SwapChainDescriptor,
        sample_count: u32,
    ) -> wgpu::TextureView {
        let multisampled_texture_extent = wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: sc_desc.format,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            label: Some("multisampled_frame"),
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CircleConstraintInstance {
    model: [[f32; 4]; 4],
}

impl CircleConstraintInstance {
    pub fn new(sphere: &Sphere) -> Self {
        let scale = cgmath::Matrix4::from_scale(sphere.radius);
        let translation = cgmath::Matrix4::from_translation(sphere.center);
        Self {
            model: (translation * scale).into(),
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<CircleConstraintInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}

pub struct TextConstraintBuilder {
    constraints: HashMap<String, Vec<Sphere>>,
    instances_cache: Option<Vec<TextConstraintInstance>>,
}

impl TextConstraintBuilder {
    pub fn new() -> Self {
        Self {
            constraints: HashMap::new(),
            instances_cache: None,
        }
    }

    pub fn with_constraint(&mut self, text: String, sphere: Sphere) {
        self.constraints
            .entry(text)
            .or_insert_with(|| Vec::with_capacity(1))
            .push(sphere);
    }

    pub fn build_instances(
        &mut self,
        glyph_brush: &mut wgpu_glyph::GlyphBrush<()>,
        view_projection_matrix: &cgmath::Matrix4<f32>,
        viewport_width: f32,
        viewport_height: f32,
        refresh_cache: bool,
    ) -> &Vec<TextConstraintInstance> {
        if self.instances_cache.is_none() || refresh_cache {
            let mut instances: Vec<TextConstraintInstance> = Vec::new();

            let mut build_onekey_instances = |text: String, spheres| {
                for sphere in spheres {
                    instances.push(TextConstraintInstance::new(
                        text.clone(),
                        glyph_brush,
                        sphere,
                        view_projection_matrix,
                        viewport_width,
                        viewport_height,
                    ));
                }
            };

            for (text, spheres) in &self.constraints {
                build_onekey_instances(text.clone(), spheres);
            }

            self.instances_cache = Some(instances);
        } else {
            for instance in self.instances_cache.as_mut().unwrap() {
                instance.set_view_projection_matrix(view_projection_matrix);
            }
        }

        self.instances_cache.as_ref().unwrap()
    }
}

pub struct TextConstraintInstance {
    text: String,
    scale: f32,
    width: f32,
    height: f32,
    sphere: Sphere,
    scaled_radius: f32,
    transformation: [f32; 16],
}

impl TextConstraintInstance {
    pub fn new(
        text: String,
        _glyph_brush: &mut wgpu_glyph::GlyphBrush<()>,
        _sphere: &Sphere,
        _view_projection_matrix: &cgmath::Matrix4<f32>,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        let mut section = wgpu_glyph::Section {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            text: vec![wgpu_glyph::Text::new(&text)
                .with_color([0.0, 0.0, 0.0, 1.0])
                .with_scale(20.0)],
            ..wgpu_glyph::Section::default()
        };
        let scaled_radius = if viewport_width > viewport_height {
            viewport_width * _sphere.radius
        } else {
            viewport_height * _sphere.radius
        };
        let (width, height) =
            Self::binary_search_for_text_scale(_glyph_brush, &mut section, scaled_radius);
        let scale = section.text[0].scale.y;
        Self {
            text: text,
            width,
            height,
            scale,
            sphere: *_sphere,
            scaled_radius,
            transformation: Self::calculate_transformation(
                _view_projection_matrix,
                _sphere,
                scaled_radius,
            ),
        }
    }

    fn set_view_projection_matrix(&mut self, view_projection_matrix: &cgmath::Matrix4<f32>) {
        self.transformation =
            Self::calculate_transformation(view_projection_matrix, &self.sphere, self.scaled_radius)
    }

    fn calculate_transformation(
        view_projection_matrix: &cgmath::Matrix4<f32>,
        sphere: &Sphere,
        scaled_radius: f32,
    ) -> [f32; 16] {
        // TODO: possible division by zero error?
        let transformation = cgmath::Matrix4::from_nonuniform_scale(
            sphere.radius / scaled_radius,
            -sphere.radius / scaled_radius,
            1.0,
        );
        let transformation = cgmath::Matrix4::from_translation(sphere.center) * transformation;
        *(view_projection_matrix * transformation).as_mut()
    }

    fn binary_search_for_text_scale(
        glyph_brush: &mut wgpu_glyph::GlyphBrush<()>,
        section: &mut wgpu_glyph::Section,
        scaled_radius: f32,
    ) -> (f32, f32) {
        use wgpu_glyph::ab_glyph::PxScale;

        let mut min_scale: PxScale = 0.0.into();
        let mut max_scale: PxScale = scaled_radius.into();
        let mut previous_scale: Option<PxScale> = None;
        let mut current_scale = (min_scale.y * 0.5 + max_scale.y * 0.5).into();
        let mut width = 0.0;
        let mut height = 0.0;
        let target = ((2.0 * scaled_radius).powf(2.0) * 0.5).sqrt();

        section.text[0].scale = current_scale;

        // Perform a binary search between [min_scale, max_scale] for the
        // correct text scale. We stop our search when the difference between
        // our previous and current text scale is small enough to not effect its
        // bounding box (i.e., the bounding box drawn from the current text
        // scale has the same dimensions as the bounding box drawn from the
        // previous text scale).
        while Some(current_scale) != previous_scale {
            match glyph_brush.glyph_bounds(&section.clone()) {
                Some(rect) => {
                    previous_scale = Some(current_scale);
                    let rect_width = rect.width();
                    let rect_height = rect.height();
                    width = rect_width;
                    height = rect_height;
                    let max_dimension = rect_width.max(rect_height);
                    if max_dimension > target {
                        max_scale = current_scale;
                    } else {
                        min_scale = current_scale;
                    }
                    current_scale = (min_scale.y * 0.5 + max_scale.y * 0.5).into();
                    section.text[0].scale = current_scale;
                }
                None => break,
            }
        }

        (width, height)
    }
}

pub struct TextConstraintRenderer<'b> {
    pub text_constraint_instances: &'b Vec<TextConstraintInstance>,
    pub device: &'b mut wgpu::Device,
    pub glyph_brush: &'b mut wgpu_glyph::GlyphBrush<()>,
    pub encoder: &'b mut wgpu::CommandEncoder,
    pub staging_belt: &'b mut wgpu::util::StagingBelt,
    pub texture_view: &'b wgpu::TextureView,
}

impl<'b> TextConstraintRenderer<'b> {
    pub fn render(&mut self) {
        for instance in self.text_constraint_instances {
            // Don't draw text that is too small to be seen clearly.
            if instance.scale > 5.0 {
                let section = wgpu_glyph::Section {
                    screen_position: (-instance.width * 0.5, -instance.height * 0.5),
                    bounds: (f32::INFINITY, f32::INFINITY),
                    text: vec![wgpu_glyph::Text::new(&instance.text)
                        .with_color([1.0, 1.0, 1.0, 1.0])
                        .with_scale(instance.scale)],
                    ..wgpu_glyph::Section::default()
                };
                self.glyph_brush.queue(&section);
                self.glyph_brush
                    .draw_queued_with_transform(
                        self.device,
                        self.staging_belt,
                        self.encoder,
                        self.texture_view,
                        instance.transformation,
                    )
                    .unwrap(); // It seems like this function always returns Ok(())...?
            }
        }
    }
}
