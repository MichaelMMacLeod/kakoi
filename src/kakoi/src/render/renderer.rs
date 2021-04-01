use std::collections::VecDeque;

use petgraph::{graph::NodeIndex, Direction};

use crate::{
    camera::Camera,
    circle::{Circle, CirclePositioner, Point},
    flat_graph::{Branch, Edge, FlatGraph, Node},
    sphere::Sphere,
};

use super::{
    builder::Builder,
    circle::{CircleConstraintBuilder, MIN_RADIUS},
    indication_tree::Tree,
    text::TextConstraintBuilder,
};

pub trait InstanceRenderer<D> {
    fn new<'a>(
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
        selected_sphere: &'a Sphere,
    ) -> Self;

    fn with_instance<'a>(&mut self, bounds: Sphere, data: &'a D);

    fn update<'a>(
        &mut self,
        queue: &'a mut wgpu::Queue,
        view_projection_matrix: &'a cgmath::Matrix4<f32>,
        selected_sphere: &'a Sphere,
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
    camera: Camera,
    width: f32,
    height: f32,
    selected_sphere: Sphere,
    selected_index: NodeIndex<u32>,
    selected_node_history: Vec<(NodeIndex<u32>, Sphere)>,
    view_projection_matrix: cgmath::Matrix4<f32>,
    text_renderer: TextConstraintBuilder,
    circle_renderer: CircleConstraintBuilder,
    cursor_position: (f32, f32),
    builder: Builder,
}

impl Renderer {
    pub fn new<'a>(device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) -> Self {
        let camera = Camera::new(sc_desc.width as f32 / sc_desc.height as f32);
        let view_projection_matrix = camera.build_view_projection_matrix();
        let mut flat_graph = FlatGraph::double_cycle_example();
        let selected_index = flat_graph.focused.unwrap();
        let selected_sphere = Sphere {
            center: cgmath::Vector3::new(0.0, 0.0, 0.0),
            radius: 1.0,
        };
        let mut circle_renderer = CircleConstraintBuilder::new(
            device,
            sc_desc,
            &view_projection_matrix,
            &selected_sphere,
        );
        let mut text_renderer =
            TextConstraintBuilder::new(device, sc_desc, &view_projection_matrix, &selected_sphere);
        let builder = Builder::new(
            &flat_graph,
            sc_desc.width as f32 / sc_desc.height as f32,
            &mut circle_renderer,
            &mut text_renderer,
        );
        // Self::build_instances(&mut flat_graph, &mut circle_renderer, &mut text_renderer);
        Self {
            flat_graph,
            camera,
            view_projection_matrix,
            text_renderer,
            circle_renderer,
            selected_sphere,
            selected_index,
            selected_node_history: Vec::new(),
            cursor_position: (0.0, 0.0),
            width: sc_desc.width as f32,
            height: sc_desc.height as f32,
            builder,
        }
    }

    pub fn update<'a>(&mut self, queue: &'a mut wgpu::Queue) {
        let aspect_corrected_sphere = Sphere {
            center: self.selected_sphere.center,
            radius: if self.camera.aspect > 1.0 {
                self.selected_sphere.radius * self.camera.aspect
            } else {
                self.selected_sphere.radius / self.camera.aspect
            },
        };

        self.circle_renderer.update(
            queue,
            &self.view_projection_matrix,
            &aspect_corrected_sphere,
        );
        self.text_renderer.update(
            queue,
            &self.view_projection_matrix,
            &aspect_corrected_sphere,
        );
    }

    pub fn resize<'a>(&mut self, device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) {
        self.width = sc_desc.width as f32;
        self.height = sc_desc.height as f32;
        self.camera.aspect = sc_desc.width as f32 / sc_desc.height as f32;
        self.view_projection_matrix = self.camera.build_view_projection_matrix();
        self.circle_renderer
            .resize(device, sc_desc, &self.view_projection_matrix);
        self.text_renderer
            .resize(device, sc_desc, &self.view_projection_matrix);
    }

    pub fn render<'a>(
        &mut self,
        device: &'a wgpu::Device,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        command_encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
    ) {
        self.circle_renderer.render(
            device,
            sc_desc,
            command_encoder,
            texture_view,
            &self.view_projection_matrix,
        );
        self.text_renderer.render(
            device,
            sc_desc,
            command_encoder,
            texture_view,
            &self.view_projection_matrix,
        );
    }

    pub fn post_render(&mut self) {
        self.circle_renderer.post_render();
        self.text_renderer.post_render();
    }

    pub fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::*;
        match event {
            WindowEvent::MouseInput { button, state, .. } if *state == ElementState::Pressed => {
                match button {
                    MouseButton::Left => {
                        let (cx, cy) = self.cursor_position;
                        let x = (2.0 * cx / self.width) - 1.0;
                        let y = (-2.0 * cy / self.height) + 1.0;
                        let (x, y) = if self.camera.aspect > 1.0 {
                            (x * self.camera.aspect, y)
                        } else {
                            (x, y * self.camera.aspect)
                        };
                        let (x, y) = (
                            x * self.selected_sphere.radius,
                            y * self.selected_sphere.radius,
                        );
                        let (x, y) = (
                            x + self.selected_sphere.center.x,
                            y + self.selected_sphere.center.y,
                        );
                        dbg!(x, y);

                        let indications = {
                            let mut walker = self
                                .flat_graph
                                .g
                                .neighbors_directed(self.selected_index, Direction::Outgoing)
                                .detach();

                            let mut indications = Vec::new();
                            let selected_sphere_radius = self.selected_sphere.radius;

                            while let Some((_, node)) = walker.next(&self.flat_graph.g) {
                                self.circle_renderer
                                    .constraints
                                    .get_mut(&node)
                                    .map(|spheres| {
                                        spheres.sort_unstable_by(|a, b| {
                                            b.radius.partial_cmp(&a.radius).unwrap()
                                        });
                                        spheres
                                            .iter()
                                            .find(|sphere| sphere.radius < selected_sphere_radius)
                                            .map(|sphere| indications.push((node, *sphere)));
                                    });
                            }

                            indications
                        };

                        let selected = indications.iter().find(|(_, sphere)| {
                            let sx = sphere.center.x;
                            let sy = sphere.center.y;

                            // let sx = sx / self.selected_sphere.radius;
                            // let sy = sy / self.selected_sphere.radius;

                            // let sx = sx - self.selected_sphere.center.x;
                            // let sy = sy - self.selected_sphere.center.y;

                            eprintln!("{:?}", (sx, sy));

                            let dx = x - sx;
                            let dy = y - sy;
                            let inside_rad = (dx * dx + dy * dy).sqrt() <= sphere.radius;
                            inside_rad
                        });

                        if let Some((node, sphere)) = selected {
                            self.selected_node_history
                                .push((self.selected_index, self.selected_sphere));
                            self.selected_sphere = *sphere;
                            self.selected_index = *node;

                            self.builder = Builder::new_with_selection(
                                &self.flat_graph,
                                self.camera.aspect,
                                &self.selected_sphere,
                                &mut self.circle_renderer,
                                &mut self.text_renderer,
                            );

                            true
                        } else {
                            false
                        }
                    }
                    MouseButton::Right => match self.selected_node_history.pop() {
                        Some((index, sphere)) => {
                            self.selected_sphere = sphere;
                            self.selected_index = index;
                            true
                        }
                        None => false,
                    },
                    _ => false,
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = (position.x as f32, position.y as f32);
                true
            }
            _ => false,
        }
    }

    // pub fn build_instances(
    //     flat_graph: &mut FlatGraph,
    //     circle_constraint_builder: &mut CircleConstraintBuilder,
    //     text_constraint_builder: &mut TextConstraintBuilder,
    // ) {
    //     let max_depth = 50;
    //     let min_radius = 0.0002;
    //     let mut todo = VecDeque::new();
    //     if let Some(focused_index) = flat_graph.focused {
    //         todo.push_back((focused_index, 1.0, Point { x: 0.0, y: 0.0 }, 0));
    //         circle_constraint_builder.with_instance(
    //             Sphere {
    //                 center: cgmath::Vector3::new(0.0, 0.0, 0.0),
    //                 radius: 1.0,
    //             },
    //             &focused_index,
    //         );
    //     }

    //     while let Some((index, radius, center, depth)) = todo.pop_front() {
    //         Self::build_instances_helper(
    //             circle_constraint_builder,
    //             text_constraint_builder,
    //             &mut todo,
    //             &flat_graph,
    //             index,
    //             radius,
    //             min_radius,
    //             center,
    //             depth,
    //             max_depth,
    //         );
    //     }
    // }

    // fn build_instances_helper(
    //     circle_constraint_builder: &mut CircleConstraintBuilder,
    //     text_constraint_builder: &mut TextConstraintBuilder,
    //     todo: &mut VecDeque<(NodeIndex<u32>, f32, Point, u32)>,
    //     flat_graph: &FlatGraph,
    //     index: NodeIndex<u32>,
    //     radius: f32,
    //     min_radius: f32,
    //     center: Point,
    //     depth: u32,
    //     max_depth: u32,
    // ) {
    //     if depth < max_depth && radius > min_radius {
    //         match &flat_graph.g[index] {
    //             Node::Leaf(text) => {
    //                 text_constraint_builder.with_instance(
    //                     Sphere {
    //                         center: cgmath::Vector3::new(center.x as f32, center.y as f32, 0.0),
    //                         radius,
    //                     },
    //                     text,
    //                 );
    //             }
    //             Node::Branch(Branch {
    //                 num_indications,
    //                 focused_indication,
    //                 zoom,
    //             }) => {
    //                 let focus_angle = 2.0 * std::f32::consts::PI / *num_indications as f32
    //                     * *focused_indication as f32;
    //                 let circle_positioner = CirclePositioner::new(
    //                     (radius * MIN_RADIUS) as f64,
    //                     *num_indications as u64,
    //                     *zoom as f64,
    //                     center,
    //                     focus_angle as f64,
    //                 );

    //                 let mut indications = {
    //                     let mut walker = flat_graph
    //                         .g
    //                         .neighbors_directed(index, Direction::Outgoing)
    //                         .detach();
    //                     let mut indications = Vec::with_capacity(*num_indications as usize);

    //                     while let Some((edge, node)) = walker.next(&flat_graph.g) {
    //                         let Edge(n) = flat_graph.g[edge];
    //                         indications.push((n, node));
    //                     }

    //                     indications
    //                 };
    //                 indications.sort_by_key(|(n, _)| *n);

    //                 let (before_focused, focused_and_after): (Vec<_>, Vec<_>) = indications
    //                     .iter()
    //                     .partition(|(i, _)| i < focused_indication);

    //                 circle_positioner
    //                     .into_iter()
    //                     .zip(focused_and_after.iter().chain(before_focused.iter()))
    //                     .for_each(|(circle, (_, node))| {
    //                         let Circle { center, radius } = circle;
    //                         let Point { x, y } = center;

    //                         todo.push_back((*node, radius as f32, center, depth + 1));
    //                         circle_constraint_builder.with_instance(
    //                             Sphere {
    //                                 center: cgmath::Vector3::new(x as f32, y as f32, 0.0),
    //                                 radius: radius as f32,
    //                             },
    //                             node,
    //                         );
    //                     });
    //             }
    //         }
        // }
    // }
}
