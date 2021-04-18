use super::{circle::CircleConstraintBuilder, image::ImageRenderer, text::TextConstraintBuilder};
use crate::camera::Camera;
use crate::input_manager::CompleteAction;
use crate::spatial_tree::SpatialTree;
use crate::{
    arena::{Arena, ArenaKey},
    input_manager::InputManager,
};

pub struct Renderer {
    store: Arena,
    camera: Camera,
    width: f32,
    height: f32,
    selected_node_history: Vec<ArenaKey>,
    text_renderer: TextConstraintBuilder,
    circle_renderer: CircleConstraintBuilder,
    image_renderer: ImageRenderer,
    cursor_position: (f32, f32),
    indication_tree: SpatialTree,
    input_manager: InputManager,
}

impl Renderer {
    pub fn new<'a>(device: &'a wgpu::Device, sc_desc: &'a wgpu::SwapChainDescriptor) -> Self {
        let mut arena = Arena::new();
        let camera = Camera::new(sc_desc.width as f32 / sc_desc.height as f32);
        let mut circle_renderer = CircleConstraintBuilder::new(device, sc_desc);
        let mut text_renderer = TextConstraintBuilder::new(device, sc_desc);
        let mut image_renderer = ImageRenderer::new(device, sc_desc);
        let selected_key = arena.register(".").unwrap();
        let spatial_tree = SpatialTree::new(
            &arena.slot_map,
            selected_key,
            &mut text_renderer,
            &mut image_renderer,
            &mut circle_renderer,
            sc_desc.width as f32,
            sc_desc.height as f32,
        );
        let input_manager = InputManager::new();
        Self {
            store: arena,
            camera,
            width: sc_desc.width as f32,
            height: sc_desc.height as f32,
            selected_node_history: vec![],
            text_renderer,
            circle_renderer,
            image_renderer,
            cursor_position: (0.0, 0.0),
            indication_tree: spatial_tree,
            input_manager,
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
        self.text_renderer.render(
            &self.store.slot_map,
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
            &self.store.slot_map,
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

        let selected_index = self.store.register(".").unwrap();

        self.indication_tree.rebuild(
            &self.store.slot_map,
            selected_index,
            &mut self.text_renderer,
            &mut self.image_renderer,
            &mut self.circle_renderer,
            self.width,
            self.height,
        );
    }

    pub fn input<'a>(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::*;
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                // eprintln!("{:?}", input);
                let should_rebuild = match self.input_manager.input(input) {
                    Some(complete_action) => match complete_action {
                        CompleteAction::SelectRegister(register) => {
                            self.selected_node_history
                                .push(self.store.register(".").unwrap());
                            self.store
                                .bind_register_to_register_value(".".into(), register);
                            true
                        }
                        CompleteAction::BindRegisterToRegisterValue(to_be_bound, to_lookup) => {
                            if to_be_bound == "." {
                                self.selected_node_history
                                    .push(self.store.register(".").unwrap());
                            }
                            self.store
                                .bind_register_to_register_value(to_be_bound, to_lookup);
                            true
                        }
                        CompleteAction::BindRegisterToString(register, string) => {
                            self.store.bind_register_to_string(register, string);
                            true
                        }
                        CompleteAction::Back => self
                            .selected_node_history
                            .pop()
                            .map(|selected_index| {
                                self.store.bind_register(".", selected_index);
                            })
                            .is_some(),
                        CompleteAction::Registers => {
                            self.selected_node_history
                                .push(self.store.register(".").unwrap());
                            self.store.bind_register(".", self.store.register_map);
                            true
                        }
                    },
                    None => false,
                };
                if should_rebuild {
                    self.rebuild_indication_tree();
                    true
                } else {
                    false
                }
            }
            WindowEvent::MouseInput { button, state, .. } if *state == ElementState::Pressed => {
                match button {
                    MouseButton::Left => {
                        self.indication_tree
                            .click(
                                self.width,
                                self.height,
                                self.cursor_position.0,
                                self.cursor_position.1,
                            )
                            .map(|selected_index| {
                                self.selected_node_history
                                    .push(self.store.register(".").unwrap());
                                // self.selected_index = selected_index;
                                self.store.bind_register(".", selected_index);
                                self.rebuild_indication_tree();
                                true
                            })
                            .unwrap_or(false)
                        // let (cx, cy) = screen_to_view_coordinates(
                        //     self.cursor_position.0,
                        //     self.cursor_position.1,
                        //     self.width,
                        //     self.height,
                        // );

                        // let overlay_focus_tree_index = {
                        //     let overlay_focus =
                        //         self.store.get_overlay(&self.selected_index).focus();
                        //     self.store
                        //         .get_indication_tree(&self.indication_tree)
                        //         .indications
                        //         .iter()
                        //         .find(|k| {
                        //             // dbg!(self.store.get_indication_tree(k).key.index(), newstore::Key::from(self.selected_index));
                        //             self.store.get_indication_tree(k).key.index()
                        //                 == overlay_focus.index()
                        //         })
                        //         .unwrap()
                        // };

                        // let selected_node = self
                        //     .store
                        //     .get_indication_tree(overlay_focus_tree_index)
                        //     .indications
                        //     .iter()
                        //     .map(|k| self.store.get_indication_tree(k))
                        //     .collect::<Vec<_>>()
                        //     .into_iter()
                        //     .find_map(
                        //         |newstore::IndicationTree {
                        //              key: node, sphere, ..
                        //          }| {
                        //             let dx = sphere.center.x - cx;
                        //             let dy = sphere.center.y - cy;
                        //             let inside_rad = (dx * dx + dy * dy).sqrt() <= sphere.radius;

                        //             if inside_rad {
                        //                 Some(node)
                        //             } else {
                        //                 None
                        //             }
                        //         },
                        //     );

                        // if let Some(node) = selected_node {
                        //     let node = *node;
                        //     let current_focus =
                        //         *self.store.get_overlay(&self.selected_index).focus();
                        //     self.store
                        //         .overlay_indicate_focus(&self.selected_index, &node);
                        //     self.selected_node_history.push(current_focus);

                        //     self.rebuild_indication_tree();

                        //     true
                        // } else {
                        //     false
                        // }
                    }
                    // MouseButton::Right => self
                    //     .selected_node_history
                    //     .pop()
                    //     .map(|selected_index| {
                    //         self.selected_index = selected_index;
                    //         self.rebuild_indication_tree();
                    //         true
                    //     })
                    //     .unwrap_or(false),
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
