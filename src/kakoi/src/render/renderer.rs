use petgraph::{graph::NodeIndex, Direction};

use crate::{camera::Camera, flat_graph::FlatGraph, sphere::Sphere};

use super::{builder::Builder, circle::CircleConstraintBuilder, text::TextConstraintBuilder};

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
    camera: Camera,
    width: f32,
    height: f32,
    selected_index: NodeIndex<u32>,
    selected_node_history: Vec<NodeIndex<u32>>,
    view_projection_matrix: cgmath::Matrix4<f32>,
    text_renderer: TextConstraintBuilder,
    circle_renderer: CircleConstraintBuilder,
    cursor_position: (f32, f32),
    builder: Builder,
}

fn screen_to_view_coordinates(
    screen_x: f32,
    screen_y: f32,
    screen_width: f32,
    screen_height: f32,
) -> (f32, f32) {
    let aspect = screen_width / screen_height;
    let (cx, cy) = (screen_x, screen_y);
    let x = (2.0 * cx / screen_width) - 1.0;
    let y = (-2.0 * cy / screen_height) + 1.0;
    if aspect > 1.0 {
        (x * aspect, y)
    } else {
        (x, y * aspect)
    }
}

impl Renderer {
    pub fn new<'a>(device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) -> Self {
        let camera = Camera::new(sc_desc.width as f32 / sc_desc.height as f32);
        let view_projection_matrix = camera.build_view_projection_matrix();
        let flat_graph = FlatGraph::naming_example();
        let selected_index = flat_graph.focused.unwrap();
        let mut circle_renderer =
            CircleConstraintBuilder::new(device, sc_desc, &view_projection_matrix);
        let mut text_renderer =
            TextConstraintBuilder::new(device, sc_desc, &view_projection_matrix);
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
            selected_index,
            selected_node_history: Vec::new(),
            cursor_position: (0.0, 0.0),
            width: sc_desc.width as f32,
            height: sc_desc.height as f32,
            builder,
        }
    }

    pub fn update<'a>(&mut self, queue: &'a mut wgpu::Queue) {
        self.circle_renderer
            .update(queue, &self.view_projection_matrix);
        self.text_renderer
            .update(queue, &self.view_projection_matrix);
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
                        let (cx, cy) = screen_to_view_coordinates(
                            self.cursor_position.0,
                            self.cursor_position.1,
                            self.width,
                            self.height,
                        );

                        let indications = self
                            .builder
                            .indication_tree
                            .indications_of(self.builder.indication_tree.root);

                        let selected_node = indications.iter().find_map(|(sphere, node)| {
                            let dx = sphere.center.x - cx;
                            let dy = sphere.center.y - cy;
                            let inside_rad = (dx * dx + dy * dy).sqrt() <= sphere.radius;

                            if inside_rad {
                                Some(node)
                            } else {
                                None
                            }
                        });

                        if let Some(node) = selected_node {
                            self.selected_node_history.push(self.selected_index);
                            self.selected_index = *node;

                            self.circle_renderer.invalidate();
                            self.text_renderer.invalidate();

                            self.builder = Builder::new_with_selection(
                                &self.flat_graph,
                                self.camera.aspect,
                                self.selected_index,
                                &mut self.circle_renderer,
                                &mut self.text_renderer,
                            );

                            true
                        } else {
                            false
                        }
                    }
                    MouseButton::Right => match self.selected_node_history.pop() {
                        Some(index) => {
                            self.selected_index = index;

                            self.circle_renderer.invalidate();
                            self.text_renderer.invalidate();

                            self.builder = Builder::new_with_selection(
                                &self.flat_graph,
                                self.camera.aspect,
                                self.selected_index,
                                &mut self.circle_renderer,
                                &mut self.text_renderer,
                            );

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
}
