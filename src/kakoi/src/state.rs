use crate::camera::Camera;
use crate::circle::{Circle, CirclePositioner, Point};
use crate::flat_graph::{Edge, FlatGraph, Node};
use crate::render;
use petgraph::{graph::NodeIndex, Direction};
use std::collections::VecDeque;
use wgpu::util::DeviceExt;
use winit::window::Window;

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    camera: Camera,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    vertex_buffer_data: Vec<Vertex>,
    staging_belt: wgpu::util::StagingBelt,
    local_pool: futures::executor::LocalPool,
    local_spawner: futures::executor::LocalSpawner,
    glyph_brush: wgpu_glyph::GlyphBrush<()>,
    text_constraint_builder: render::TextConstraintBuilder,
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

const MIN_RADIUS: f32 = 0.98;
const MAX_RADIUS: f32 = 1.0;

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

struct TextLeaf<'a> {
    bounding_square_center: cgmath::Vector3<f32>,
    bounding_square_size: f32,
    text_width: f32,
    text_height: f32,
    text: &'a String,
}

impl<'a> TextLeaf<'a> {
    fn get_projection(&self, camera: &Camera, size: f32) -> [f32; 16] {
        let t = cgmath::Matrix4::from_nonuniform_scale(2.0 / size, 2.0 / size, 1.0);
        let t = cgmath::Matrix4::from_nonuniform_scale(1.0, -1.0, 1.0) * t;
        let t = cgmath::Matrix4::from_scale(self.bounding_square_size) * t;
        let mut t = cgmath::Matrix4::from_translation(
            self.bounding_square_center
                - cgmath::Vector3::new(1.0, -1.0, 0.0) * self.bounding_square_size / 2.0,
        ) * t;
        *(camera.build_view_projection_matrix() * t).as_mut()
    }

    fn get_text(&self) -> wgpu_glyph::Text {
        wgpu_glyph::Text::new(self.text).with_color([1.0, 1.0, 1.0, 1.0])
    }
}

#[derive(Debug)]
struct Instance {
    position: cgmath::Vector3<f32>,
    radius: f32,
}

impl Instance {
    fn new(x: f32, y: f32, radius: f32) -> Self {
        Self {
            position: cgmath::Vector3::new(x, y, 0.0), // TODO
            radius,
        }
    }
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        let scale = cgmath::Matrix4::from_scale(self.radius);
        let translation = cgmath::Matrix4::from_translation(self.position);
        InstanceRaw {
            model: (translation * scale).into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as wgpu::BufferAddress,
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

impl State {
    fn build_instances(
        text_constraint_builder: &mut render::TextConstraintBuilder,
    ) -> Vec<Instance> {
        let flat_graph = FlatGraph::naming_example();

        let max_depth = 50;
        let min_radius = 0.0002;
        let mut instances = Vec::new();
        let mut todo = VecDeque::new();
        if let Some(focused_index) = flat_graph.focused {
            todo.push_back((focused_index, 1.0, Point { x: 0.0, y: 0.0 }, 0));
            instances.push(Instance::new(0.0, 0.0, 1.0));
        }
        // instances.push(Instance::new(-0.5, 0.0, 0.5));
        // instances.push(Instance::new(0.5, 0.0, 0.5));

        while let Some((index, radius, center, depth)) = todo.pop_front() {
            Self::build_instances_helper(
                text_constraint_builder,
                &mut instances,
                &mut todo,
                &flat_graph,
                index,
                radius,
                min_radius,
                center,
                depth,
                max_depth,
            );
        }

        instances
    }

    fn build_instances_helper(
        text_constraint_builder: &mut render::TextConstraintBuilder,
        instances: &mut Vec<Instance>,
        todo: &mut VecDeque<(NodeIndex<u32>, f32, Point, u32)>,
        flat_graph: &FlatGraph,
        index: NodeIndex<u32>,
        radius: f32,
        min_radius: f32,
        center: Point,
        depth: u32,
        max_depth: u32,
    ) {
        if depth < max_depth && radius > min_radius {
            match &flat_graph.g[index] {
                Node::Leaf(text) => {
                    text_constraint_builder.with_constraint(
                        text.clone(),
                        render::Sphere {
                            center: cgmath::Vector3::new(center.x as f32, center.y as f32, 0.0),
                            radius,
                        },
                    );
                }
                Node::Branch(num_indications) => {
                    let circle_positioner = CirclePositioner::new(
                        (radius * MIN_RADIUS) as f64,
                        *num_indications as u64,
                        0.0,
                        center,
                        0.0,
                    );

                    let mut indications = {
                        let mut walker = flat_graph
                            .g
                            .neighbors_directed(index, Direction::Outgoing)
                            .detach();
                        let mut indications = Vec::with_capacity(*num_indications as usize);

                        while let Some((edge, node)) = walker.next(&flat_graph.g) {
                            let Edge(n) = flat_graph.g[edge];
                            indications.push((n, node));
                        }

                        indications
                    };
                    indications.sort_by_key(|(n, _)| *n);

                    circle_positioner
                        .into_iter()
                        .zip(indications.iter())
                        .for_each(|(circle, (_, node))| {
                            let Circle { center, radius } = circle;
                            let Point { x, y } = center;

                            todo.push_back((*node, radius as f32, center, depth + 1));
                            instances.push(Instance::new(x as f32, y as f32, radius as f32));
                        });
                }
            }
        }
    }

    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let texture_format = adapter.get_swap_chain_preferred_format(&surface);

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: texture_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let vertex_buffer_data = Vertex::circle();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_buffer_data),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let mut text_constraint_builder = render::TextConstraintBuilder::new();

        let instances = Self::build_instances(&mut text_constraint_builder);
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/build/shader.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/build/shader.frag.spv"));

        let camera = Camera::new(sc_desc.width as f32 / sc_desc.height as f32);

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

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
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    alpha_blend: wgpu::BlendState {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    color_blend: wgpu::BlendState {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
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

        // Not exactly sure what size to set here. Smaller sizes (~1024) seem to
        // cause lag. Larger sizes (~4096) seem to cause less lag. Ideally, we'd
        // base this number on an estimate of how much data we would upload into
        // it. See https://docs.rs/wgpu/0.7.0/wgpu/util/struct.StagingBelt.html
        // for more information.
        let staging_belt = wgpu::util::StagingBelt::new(4096);

        let local_pool = futures::executor::LocalPool::new();
        let local_spawner = local_pool.spawner();

        let glyph_brush = {
            let font = wgpu_glyph::ab_glyph::FontArc::try_from_slice(include_bytes!(
                "resources/fonts/CooperHewitt-OTF-public/CooperHewitt-Book.otf"
            ))
            .unwrap();
            wgpu_glyph::GlyphBrushBuilder::using_font(font).build(&device, texture_format)
        };

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline,
            vertex_buffer,
            instances,
            instance_buffer,
            camera,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            vertex_buffer_data,
            staging_belt,
            local_pool,
            local_spawner,
            glyph_brush,
            text_constraint_builder,
        }
    }

    pub fn recreate_swap_chain(&mut self) {
        self.resize(self.size);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::*;
        match event {
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, y),
                ..
            } => {
                self.camera.eye.z *= 1.0 + 0.1 * y;
                true
            }
            WindowEvent::CursorMoved {
                position: winit::dpi::PhysicalPosition { x, y },
                ..
            } => {
                self.camera.eye.x = *x as f32 / self.sc_desc.width as f32 - 0.5;
                self.camera.eye.y = -*y as f32 / self.sc_desc.height as f32 + 0.5;
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self) {
        self.uniforms.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            let width = self.sc_desc.width as f32;
            let height = self.sc_desc.height as f32;
            render_pass.set_viewport(0.0, 0.0, width, height, 0.0, 1.0);
            render_pass.draw(
                0..self.vertex_buffer_data.len() as _,
                0..self.instances.len() as _,
            );
        }

        let text_constraint_instances = self.text_constraint_builder.build_instances(
            &mut self.glyph_brush,
            &self.camera.build_view_projection_matrix(),
            self.sc_desc.width as f32,
            self.sc_desc.height as f32,
            false,
        );
        let mut text_constraint_renderer = render::TextConstraintRenderer {
            text_constraint_instances,
            device: &mut self.device,
            glyph_brush: &mut self.glyph_brush,
            encoder: &mut encoder,
            staging_belt: &mut self.staging_belt,
            texture_view: &frame.view,
        };
        text_constraint_renderer.render();

        self.staging_belt.finish();

        self.queue.submit(std::iter::once(encoder.finish()));

        use futures::task::SpawnExt;

        self.local_spawner
            .spawn(self.staging_belt.recall())
            .expect("Recall staging belt");

        self.local_pool.run_until_stalled();

        Ok(())
    }
}

#[cfg(test)]
mod test {

    // #[test]
    // fn magic0() {
    //     let verts = Vertex::make_circle(4, 0.9, 1.0);
    //     let mut x_vals = Vec::new();
    //     let mut y_vals = Vec::new();
    //     for Vertex {
    //         position: [x, y, _],
    //     } in verts
    //     {
    //         x_vals.push(x);
    //         y_vals.push(y);
    //     }
    //     eprint!("[");
    //     for x in x_vals {
    //         eprint!("{},", x)
    //     }
    //     eprintln!("]");
    //     eprint!("[");
    //     for y in y_vals {
    //         eprint!("{},", y)
    //     }
    //     eprintln!("]");
    //     panic!();
    // }

    // #[test]
    // fn magic1() {
    //     let mut u = Uniforms::new();
    //     let camera = Camera {
    //         eye: (0.0, 1.0, 2.0).into(),
    //         target: (0.0, 0.0, 0.0).into(),
    //         up: cgmath::Vector3::unit_y(),
    //         aspect: 600.0 / 600.0,
    //         fovy: 45.0,
    //         znear: 0.1,
    //         zfar: 100.0,
    //     };
    //     u.update_view_proj(&camera);

    //     let i = Instance::new(2.3, 4.5, 100.0);
    //     let raw_i = i.to_raw();
    //     /*
    //     1.0 0.0 0.0 2.3
    //     0.0 1.0 0.0 4.5
    //     0.0 0.0 1.0 0.0
    //     0.0 0.0 0.0 1.0
    //     */
    //     dbg!(cgmath::Matrix4::from(u.view_proj) * cgmath::Matrix4::from(raw_i.model));
    //     panic!();
    // }

    #[test]
    fn magic2() {
        use super::*;
        let leaf = TextLeaf {
            bounding_square_center: cgmath::Vector3::new(0.0, 0.0, 0.0),
            bounding_square_size: 1.0,
            text_width: 185.89133,
            text_height: 300.0,
            text: &"Hello, world!".into(),
        };
        dbg!(leaf.get_projection(&Camera::new(1.0), 600.0));
    }
}
