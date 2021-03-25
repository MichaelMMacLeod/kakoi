use crate::camera::Camera;
use crate::circle::{Circle, CirclePositioner, Point};
use crate::flat_graph::{Branch, Edge, FlatGraph, Node};
use crate::render;
use crate::sphere::Sphere;
use petgraph::{graph::NodeIndex, Direction};
use std::collections::VecDeque;
use winit::window::Window;

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    camera: Camera,
    staging_belt: wgpu::util::StagingBelt,
    local_pool: futures::executor::LocalPool,
    local_spawner: futures::executor::LocalSpawner,
    glyph_brush: wgpu_glyph::GlyphBrush<()>,
    circle_constraint_builder: render::circle::CircleConstraintBuilder,
    text_constraint_builder: render::text::TextConstraintBuilder,
}

#[derive(Debug)]
struct Instance {
    position: cgmath::Vector3<f32>,
    radius: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl State {
    fn build_instances(
        circle_constraint_builder: &mut render::circle::CircleConstraintBuilder,
        text_constraint_builder: &mut render::text::TextConstraintBuilder,
    ) {
        let flat_graph = FlatGraph::naming_example();

        let max_depth = 50;
        let min_radius = 0.0002;
        let mut todo = VecDeque::new();
        if let Some(focused_index) = flat_graph.focused {
            todo.push_back((focused_index, 1.0, Point { x: 0.0, y: 0.0 }, 0));
            circle_constraint_builder.with_constraint(
                focused_index,
                Sphere {
                    center: cgmath::Vector3::new(0.0, 0.0, 0.0),
                    radius: 1.0,
                },
            );
        }

        while let Some((index, radius, center, depth)) = todo.pop_front() {
            Self::build_instances_helper(
                circle_constraint_builder,
                text_constraint_builder,
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
    }

    fn build_instances_helper(
        circle_constraint_builder: &mut render::circle::CircleConstraintBuilder,
        text_constraint_builder: &mut render::text::TextConstraintBuilder,
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
                        Sphere {
                            center: cgmath::Vector3::new(center.x as f32, center.y as f32, 0.0),
                            radius,
                        },
                    );
                }
                Node::Branch(Branch {
                    num_indications,
                    focused_indication,
                    zoom,
                }) => {
                    let focus_angle = 2.0 * std::f32::consts::PI / *num_indications as f32
                        * *focused_indication as f32;
                    let circle_positioner = CirclePositioner::new(
                        (radius * render::circle::MIN_RADIUS) as f64,
                        *num_indications as u64,
                        *zoom as f64,
                        center,
                        focus_angle as f64,
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

                    let (before_focused, focused_and_after): (Vec<_>, Vec<_>) = indications
                        .iter()
                        .partition(|(i, _)| i < focused_indication);

                    circle_positioner
                        .into_iter()
                        .zip(focused_and_after.iter().chain(before_focused.iter()))
                        .for_each(|(circle, (_, node))| {
                            let Circle { center, radius } = circle;
                            let Point { x, y } = center;

                            todo.push_back((*node, radius as f32, center, depth + 1));
                            circle_constraint_builder.with_constraint(
                                index,
                                Sphere {
                                    center: cgmath::Vector3::new(x as f32, y as f32, 0.0),
                                    radius: radius as f32,
                                },
                            );
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

        let mut text_constraint_builder = render::text::TextConstraintBuilder::new();
        let mut circle_constraint_builder = render::circle::CircleConstraintBuilder::new(&device, &sc_desc);

        Self::build_instances(&mut circle_constraint_builder, &mut text_constraint_builder);

        let camera = Camera::new(sc_desc.width as f32 / sc_desc.height as f32);

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
            camera,
            staging_belt,
            local_pool,
            local_spawner,
            glyph_brush,
            circle_constraint_builder,
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
        self.circle_constraint_builder
            .resize(&self.device, &self.sc_desc);
    }

    pub fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::*;
        match event {
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, _y),
                ..
            } => {
                // self.camera.eye.z *= 1.0 + 0.1 * y;
                true
            }
            WindowEvent::CursorMoved {
                position: winit::dpi::PhysicalPosition { x: _x, y: _y },
                ..
            } => {
                // self.camera.eye.x = *x as f32 / self.sc_desc.width as f32 - 0.5;
                // self.camera.eye.y = -*y as f32 / self.sc_desc.height as f32 + 0.5;
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self) {
        self.circle_constraint_builder
            .update(&mut self.queue, &self.camera);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.circle_constraint_builder.render(
            &self.device,
            &mut encoder,
            &frame.view,
            &self.sc_desc,
        );

        let text_constraint_instances = self.text_constraint_builder.build_instances(
            &mut self.glyph_brush,
            &self.camera.build_view_projection_matrix(),
            self.sc_desc.width as f32,
            self.sc_desc.height as f32,
            false,
        );
        let mut text_constraint_renderer = render::text::TextConstraintRenderer {
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
