use crate::{camera::Camera, flat_graph, store};

use super::{
    builder::Builder, circle::CircleConstraintBuilder, image::ImageRenderer,
    text::TextConstraintBuilder,
};

pub struct Renderer {
    store: store::Store,
    #[allow(unused)]
    flat_graph: flat_graph::FlatGraph,
    camera: Camera,
    width: f32,
    height: f32,
    selected_index: store::Key,
    selected_node_history: Vec<store::Key>,
    text_renderer: TextConstraintBuilder,
    circle_renderer: CircleConstraintBuilder,
    image_renderer: ImageRenderer,
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
        (x, y / aspect)
    }
}

impl Renderer {
    pub fn new<'a>(
        device: &'a wgpu::Device,
        queue: &'a mut wgpu::Queue,
        sc_desc: &'a wgpu::SwapChainDescriptor,
    ) -> Self {
        let mut store = store::Store::new();
        let camera = Camera::new(sc_desc.width as f32 / sc_desc.height as f32);
        let mut graph = flat_graph::FlatGraph::naming_example(&mut store);
        let message = graph
            .enclose(
                &mut store,
                flat_graph::Group::New,
                &mut [flat_graph::Insertion::New(store::Value::String(
                    "Welcome to\nKAKOI!".into(),
                ))]
                .into(),
            )
            .unwrap();
        let overlay = {
            let focused = graph.focused.unwrap();
            store::Overlay::new(&mut store, &mut graph, focused, message)
        };
        let mut circle_renderer = CircleConstraintBuilder::new(device, sc_desc);
        let mut text_renderer = TextConstraintBuilder::new(device, sc_desc);
        let mut image_renderer = ImageRenderer::new(device, sc_desc);
        let builder = Builder::new(
            device,
            queue,
            &store,
            overlay,
            sc_desc.width as f32,
            sc_desc.height as f32,
            &mut circle_renderer,
            &mut text_renderer,
            &mut image_renderer,
        );
        Self {
            store,
            flat_graph: graph,
            camera,
            text_renderer,
            circle_renderer,
            image_renderer,
            selected_index: overlay,
            selected_node_history: Vec::new(),
            cursor_position: (0.0, 0.0),
            width: sc_desc.width as f32,
            height: sc_desc.height as f32,
            builder,
        }
    }

    pub fn resize<'a>(
        &mut self,
        device: &'a wgpu::Device,
        queue: &'a mut wgpu::Queue,
        sc_desc: &'a wgpu::SwapChainDescriptor,
    ) {
        self.width = sc_desc.width as f32;
        self.height = sc_desc.height as f32;
        self.camera
            .set_aspect(sc_desc.width as f32 / sc_desc.height as f32);
        self.circle_renderer.resize();
        self.text_renderer.resize();
        self.image_renderer.resize();
        self.circle_renderer.invalidate();
        self.text_renderer.invalidate();
        self.image_renderer.invalidate();
        self.builder = Builder::new_with_selection(
            device,
            queue,
            &self.store,
            self.width,
            self.height,
            self.selected_index,
            &mut self.circle_renderer,
            &mut self.text_renderer,
            &mut self.image_renderer,
        );
    }

    pub fn render<'a>(
        &mut self,
        device: &'a wgpu::Device,
        queue: &'a mut wgpu::Queue,
        sc_desc: &'a wgpu::SwapChainDescriptor,
        command_encoder: &'a mut wgpu::CommandEncoder,
        texture_view: &'a wgpu::TextureView,
    ) {
        self.circle_renderer.render(
            device,
            queue,
            sc_desc,
            command_encoder,
            texture_view,
            &mut self.camera,
        );
        self.text_renderer.render(
            &self.store,
            device,
            sc_desc,
            command_encoder,
            texture_view,
            &mut self.camera,
        );
        self.image_renderer.render(
            device,
            queue,
            command_encoder,
            texture_view,
            &mut self.camera,
        );
    }

    pub fn post_render(&mut self) {
        self.circle_renderer.post_render();
        self.text_renderer.post_render();
    }

    pub fn input<'a>(
        &mut self,
        device: &'a wgpu::Device,
        queue: &'a mut wgpu::Queue,
        event: &winit::event::WindowEvent,
    ) -> bool {
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

                        let overlay_focus_tree_index = {
                            let overlay_focus = *self
                                .store
                                .get(&self.selected_index)
                                .unwrap()
                                .overlay()
                                .unwrap()
                                .focus();

                            let mut walker = self
                                .builder
                                .indication_tree
                                .g
                                .neighbors_directed(
                                    self.builder.indication_tree.root,
                                    petgraph::Direction::Outgoing,
                                )
                                .detach();

                            let mut result = None;

                            while let Some((_, indication)) = walker.next(&self.builder.indication_tree.g) {
                                let node = &self.builder.indication_tree.g[indication];

                                if node.key == overlay_focus {
                                    result = Some(indication);
                                    break;
                                }
                            }

                            result.unwrap()
                        };

                        let indications = self
                            .builder
                            .indication_tree
                            .indications_of(overlay_focus_tree_index);

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
                            {
                                let mut flat_graph = &mut self.flat_graph;
                                let selected_index = self.selected_index;
                                let selected_node_history = &mut self.selected_node_history;
                                self.store.entry(self.selected_index).and_modify(|value| {
                                    let overlay = value.overlay_mut().unwrap();
                                    selected_node_history.push(*overlay.focus());
                                    overlay.set_focus(&mut flat_graph, selected_index, *node);
                                });
                            }

                            self.circle_renderer.invalidate();
                            self.text_renderer.invalidate();
                            self.image_renderer.invalidate();

                            self.builder = Builder::new_with_selection(
                                device,
                                queue,
                                &self.store,
                                self.width,
                                self.height,
                                self.selected_index,
                                &mut self.circle_renderer,
                                &mut self.text_renderer,
                                &mut self.image_renderer,
                            );

                            true
                        } else {
                            false
                        }
                    }
                    MouseButton::Right => match self.selected_node_history.pop() {
                        Some(index) => {
                            {
                                let mut flat_graph = &mut self.flat_graph;
                                let selected_index = self.selected_index;
                                self.store.entry(self.selected_index).and_modify(|value| {
                                    let overlay = value.overlay_mut().unwrap();
                                    overlay.set_focus(&mut flat_graph, selected_index, index);
                                });
                            }

                            self.circle_renderer.invalidate();
                            self.text_renderer.invalidate();
                            self.image_renderer.invalidate();

                            self.builder = Builder::new_with_selection(
                                device,
                                queue,
                                &self.store,
                                self.width,
                                self.height,
                                self.selected_index,
                                &mut self.circle_renderer,
                                &mut self.text_renderer,
                                &mut self.image_renderer,
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
