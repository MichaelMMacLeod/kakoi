use std::collections::VecDeque;

use petgraph::{graph::NodeIndex, Direction};

use crate::{
    circle::{Circle, CirclePositioner, Point},
    flat_graph::{Branch, Edge, FlatGraph, Node},
    sphere::Sphere,
};

use super::{
    circle::{CircleConstraintBuilder, MIN_RADIUS},
    text::TextConstraintBuilder,
};

pub trait InstanceRenderer<D> {
    fn new<'a>(
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    ) -> Self;

    fn with_instance<'a>(&mut self, bounds: Sphere, data: &'a D);

    fn update<'a>(
        &mut self,
        queue: &'a mut wgpu::Queue,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    );

    fn resize<'a>(
        &mut self,
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    );

    fn render<'a>(
        &mut self,
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        command_encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    );

    fn post_render(&mut self);
}

pub struct Renderer {
    flat_graph: FlatGraph,
    text_renderer: TextConstraintBuilder,
    circle_renderer: CircleConstraintBuilder,
}

impl Renderer {
    pub fn new<'a>(
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    ) -> Self {
        let mut flat_graph = FlatGraph::naming_example();
        let mut circle_renderer =
            CircleConstraintBuilder::new(device, sc_desc, view_projection_matrix);
        let mut text_renderer = TextConstraintBuilder::new(device, sc_desc, view_projection_matrix);
        Self::build_instances(&mut flat_graph, &mut circle_renderer, &mut text_renderer);
        Self {
            flat_graph,
            text_renderer,
            circle_renderer,
        }
    }

    pub fn update<'a>(
        &mut self,
        queue: &'a mut wgpu::Queue,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    ) {
        self.circle_renderer.update(queue, view_projection_matrix);
        self.text_renderer.update(queue, view_projection_matrix);
    }

    pub fn resize<'a>(
        &mut self,
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    ) {
        self.circle_renderer
            .resize(device, sc_desc, view_projection_matrix);
        self.text_renderer
            .resize(device, sc_desc, view_projection_matrix);
    }

    pub fn render<'a>(
        &mut self,
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        command_encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
    ) {
        self.circle_renderer.render(
            device,
            sc_desc,
            command_encoder,
            texture_view,
            view_projection_matrix,
        );
        self.text_renderer.render(
            device,
            sc_desc,
            command_encoder,
            texture_view,
            view_projection_matrix,
        );
    }

    pub fn post_render(&mut self) {
        self.circle_renderer.post_render();
        self.text_renderer.post_render();
    }

    pub fn build_instances(
        flat_graph: &mut FlatGraph,
        circle_constraint_builder: &mut CircleConstraintBuilder,
        text_constraint_builder: &mut TextConstraintBuilder,
    ) {
        let max_depth = 50;
        let min_radius = 0.0002;
        let mut todo = VecDeque::new();
        if let Some(focused_index) = flat_graph.focused {
            todo.push_back((focused_index, 1.0, Point { x: 0.0, y: 0.0 }, 0));
            circle_constraint_builder.with_instance(
                Sphere {
                    center: cgmath::Vector3::new(0.0, 0.0, 0.0),
                    radius: 1.0,
                },
                &focused_index,
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
        circle_constraint_builder: &mut CircleConstraintBuilder,
        text_constraint_builder: &mut TextConstraintBuilder,
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
                    text_constraint_builder.with_instance(
                        Sphere {
                            center: cgmath::Vector3::new(center.x as f32, center.y as f32, 0.0),
                            radius,
                        },
                        text,
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
                        (radius * MIN_RADIUS) as f64,
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
                            circle_constraint_builder.with_instance(
                                Sphere {
                                    center: cgmath::Vector3::new(x as f32, y as f32, 0.0),
                                    radius: radius as f32,
                                },
                                &index,
                            );
                        });
                }
            }
        }
    }
}
