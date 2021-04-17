use crate::{camera::Camera, newstore};

use super::{circle::CircleConstraintBuilder, image::ImageRenderer, text::TextConstraintBuilder};

pub struct Renderer {
    store: newstore::Store,
    camera: Camera,
    width: f32,
    height: f32,
    selected_index: newstore::OverlayKey,
    selected_node_history: Vec<newstore::Key>,
    text_renderer: TextConstraintBuilder,
    circle_renderer: CircleConstraintBuilder,
    image_renderer: ImageRenderer,
    cursor_position: (f32, f32),
    indication_tree: newstore::IndicationTreeKey,
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
    pub fn new<'a>(device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) -> Self {
        let (mut store, overlay_key) = newstore::Store::naming_example();
        let camera = Camera::new(sc_desc.width as f32 / sc_desc.height as f32);
        let mut circle_renderer = CircleConstraintBuilder::new(device, sc_desc);
        let mut text_renderer = TextConstraintBuilder::new(device, sc_desc);
        let mut image_renderer = ImageRenderer::new(device, sc_desc);
        let indication_tree_key = store.build_indication_tree(
            newstore::Key::from(overlay_key),
            sc_desc.width as f32,
            sc_desc.height as f32,
            &mut circle_renderer,
            &mut text_renderer,
            &mut image_renderer,
        );
        Self {
            store,
            camera,
            width: sc_desc.width as f32,
            height: sc_desc.height as f32,
            selected_index: overlay_key,
            selected_node_history: vec![],
            text_renderer,
            circle_renderer,
            image_renderer,
            cursor_position: (0.0, 0.0),
            indication_tree: indication_tree_key,
        }
    }

    pub fn resize<'a>(&mut self, sc_desc: &'a wgpu::SwapChainDescriptor) {
        self.width = sc_desc.width as f32;
        self.height = sc_desc.height as f32;
        self.camera
            .set_aspect(sc_desc.width as f32 / sc_desc.height as f32);

        self.circle_renderer.resize();
        self.text_renderer.resize();
        self.image_renderer.resize();

        self.rebuild_indication_tree();
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
        // self.text_renderer.render(
        //     &self.store,
        //     device,
        //     sc_desc,
        //     command_encoder,
        //     texture_view,
        //     &mut self.camera,
        // );
        self.image_renderer.render(
            device,
            queue,
            command_encoder,
            texture_view,
            &mut self.camera,
            &self.store,
        );
    }

    pub fn post_render(&mut self) {
        self.circle_renderer.post_render();
        self.text_renderer.post_render();
    }

    fn rebuild_indication_tree(&mut self) {
        self.circle_renderer.invalidate();
        self.text_renderer.invalidate();
        self.image_renderer.invalidate();

        self.store.remove_indication_tree(self.indication_tree);

        self.indication_tree = self.store.build_indication_tree(
            newstore::Key::from(self.selected_index),
            self.width,
            self.height,
            &mut self.circle_renderer,
            &mut self.text_renderer,
            &mut self.image_renderer,
        );
    }

    pub fn input<'a>(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::*;
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                eprintln!("{:?}", input);
                input.virtual_keycode.map_or(false, |virtual_key_code| {
                    if input.state == ElementState::Released {
                        let register_map = *self
                            .store
                            .get_overlay(&self.selected_index)
                            .message()
                            .map_key()
                            .unwrap();
                        let selected_key = *self.store.get_overlay(&self.selected_index).focus();
                        let register_string = format!("{:?}", virtual_key_code);
                        let register_key = if let Some(&register_key) = self.store.get_by_value(&register_string) {
                            register_key
                        } else {
                            newstore::Key::from(self.store.insert_string(&register_string))
                        };
                        self.store.map_set_key_value(
                            &register_map,
                            &register_key,
                            &selected_key,
                        );
                        self.rebuild_indication_tree();
                        true
                    } else {
                        false
                    }
                })
            }
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
                            let overlay_focus =
                                self.store.get_overlay(&self.selected_index).focus();
                            self.store
                                .get_indication_tree(&self.indication_tree)
                                .indications
                                .iter()
                                .find(|k| {
                                    // dbg!(self.store.get_indication_tree(k).key.index(), newstore::Key::from(self.selected_index));
                                    self.store.get_indication_tree(k).key.index()
                                        == overlay_focus.index()
                                })
                                .unwrap()
                        };

                        let selected_node = self
                            .store
                            .get_indication_tree(overlay_focus_tree_index)
                            .indications
                            .iter()
                            .map(|k| self.store.get_indication_tree(k))
                            .collect::<Vec<_>>()
                            .into_iter()
                            .find_map(
                                |newstore::IndicationTree {
                                     key: node, sphere, ..
                                 }| {
                                    let dx = sphere.center.x - cx;
                                    let dy = sphere.center.y - cy;
                                    let inside_rad = (dx * dx + dy * dy).sqrt() <= sphere.radius;

                                    if inside_rad {
                                        Some(node)
                                    } else {
                                        None
                                    }
                                },
                            );

                        if let Some(node) = selected_node {
                            let node = *node;
                            let current_focus =
                                *self.store.get_overlay(&self.selected_index).focus();
                            self.store
                                .overlay_indicate_focus(&self.selected_index, &node);
                            self.selected_node_history.push(current_focus);

                            self.rebuild_indication_tree();

                            true
                        } else {
                            false
                        }
                    }
                    MouseButton::Right => match self.selected_node_history.pop() {
                        Some(index) => {
                            self.store
                                .overlay_indicate_focus(&self.selected_index, &index);

                            self.rebuild_indication_tree();

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
