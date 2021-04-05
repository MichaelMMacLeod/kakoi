use std::collections::VecDeque;

use petgraph::graph::NodeIndex;

use crate::circle::{Circle, CirclePositioner};
use crate::{circle::Point, sphere::Sphere, store};

use super::{
    circle::{CircleConstraintBuilder, MIN_RADIUS},
    image::ImageRenderer,
    indication_tree::{self, Tree, TreeNode},
    text::TextConstraintBuilder,
};

pub struct Builder {
    pub indication_tree: Tree,
}

// fn indications_of<'a>(
//     flat_graph: &'a FlatGraph,
//     index: NodeIndex<u32>,
//     num_indications: usize,
// ) -> Vec<(u32, NodeIndex<u32>)> {
//     let mut walker = flat_graph
//         .g
//         .neighbors_directed(index, petgraph::Direction::Outgoing)
//         .detach();

//     let mut indications = Vec::with_capacity(num_indications);

//     while let Some((edge, node)) = walker.next(&flat_graph.g) {
//         let flat_graph::Edge(n) = flat_graph.g[edge];
//         indications.push((n, node));
//     }

//     indications
// }

fn build_indication_tree_2<'a>(
    device: &'a wgpu::Device,
    queue: &'a mut wgpu::Queue,
    store: &'a store::Store,
    tree_impl: &'a mut indication_tree::Impl,
    root_index: NodeIndex<u32>,
    screen_width: f32,
    screen_height: f32,
    circle_builder: &'a mut CircleConstraintBuilder,
    text_builder: &'a mut TextConstraintBuilder,
    image_builder: &'a mut ImageRenderer,
) {
    let mut todo = VecDeque::new();
    todo.push_back(root_index);

    while let Some(indication_tree_index) = todo.pop_front() {
        let TreeNode {
            sphere,
            // flat_graph_index,
            key,
        } = &tree_impl[NodeIndex::from(indication_tree_index)];

        match store.get(key).unwrap() {
            store::Value::String(_) => {
                text_builder.with_instance(*sphere, *key);
            }
            store::Value::Image(_) => {
                image_builder.with_image(device, queue, store, *sphere, *key);
            }
            store::Value::Association(association) => {
                let indications = &association.indications;
                let focused_indication = association.focused_indication;
                let zoom = association.zoom();
                let focus_angle = 2.0 * std::f32::consts::PI / indications.len() as f32
                    * focused_indication as f32;
                let circle_positioner = CirclePositioner::new(
                    (sphere.radius * MIN_RADIUS) as f64,
                    indications.len() as u64,
                    zoom as f64,
                    Point {
                        x: sphere.center.x as f64,
                        y: sphere.center.y as f64,
                    },
                    focus_angle as f64,
                );
                let (before_focused, focused_and_after): (Vec<_>, Vec<_>) = (0..)
                    .into_iter()
                    .zip(indications.iter())
                    .partition(|(i, _)| *i < focused_indication);
                circle_positioner
                    .into_iter()
                    .zip(focused_and_after.iter().chain(before_focused.iter()))
                    .for_each(|(circle, (_, node))| {
                        let Circle { center, radius } = circle;
                        let Point { x, y } = center;
                        let radius = radius as f32;

                        let other_sphere = Sphere {
                            center: cgmath::vec3(x as f32, y as f32, 0.0),
                            radius,
                        };

                        if other_sphere.screen_radius(screen_width, screen_height) > 1.0 {
                            let indicated_index = tree_impl.add_node(indication_tree::TreeNode {
                                key: node.target,
                                sphere: other_sphere,
                            });
                            tree_impl.add_edge(indication_tree_index, indicated_index, ());
                            todo.push_back(indicated_index);

                            circle_builder.with_instance(other_sphere);
                        }
                    });
            }
        }
    }
}

fn build_indication_tree_1<'a>(
    device: &'a wgpu::Device,
    queue: &'a mut wgpu::Queue,
    store: &'a store::Store,
    screen_width: f32,
    screen_height: f32,
    selected_key: store::Key,
    circle_builder: &'a mut CircleConstraintBuilder,
    text_builder: &'a mut TextConstraintBuilder,
    image_builder: &'a mut ImageRenderer,
) -> Tree {
    let mut tree_impl = indication_tree::Impl::new();

    let first_indication_tree_index = {
        let first_sphere = Sphere {
            center: cgmath::vec3(0.0, 0.0, 0.0),
            radius: 1.0,
        };
        circle_builder.with_instance(first_sphere);
        tree_impl
            .add_node(TreeNode {
                key: selected_key,
                sphere: first_sphere,
            })
            .into()
    };

    let mut todo = VecDeque::new();
    todo.push_back(first_indication_tree_index);

    build_indication_tree_2(
        device,
        queue,
        store,
        &mut tree_impl,
        first_indication_tree_index,
        screen_width,
        screen_height,
        circle_builder,
        text_builder,
        image_builder,
    );

    Tree {
        g: tree_impl,
        root: first_indication_tree_index,
    }
}

impl Builder {
    pub fn new<'a>(
        device: &'a wgpu::Device,
        queue: &'a mut wgpu::Queue,
        store: &'a store::Store,
        selected_node: store::Key,
        screen_width: f32,
        screen_height: f32,
        circle_builder: &'a mut CircleConstraintBuilder,
        text_builder: &'a mut TextConstraintBuilder,
        image_builder: &'a mut ImageRenderer,
    ) -> Self {
        Self::new_with_selection(
            device,
            queue,
            store,
            screen_width,
            screen_height,
            selected_node,
            circle_builder,
            text_builder,
            image_builder,
        )
    }

    pub fn new_with_selection<'a>(
        device: &'a wgpu::Device,
        queue: &'a mut wgpu::Queue,
        store: &'a store::Store,
        screen_width: f32,
        screen_height: f32,
        selected_node: store::Key,
        circle_builder: &'a mut CircleConstraintBuilder,
        text_builder: &'a mut TextConstraintBuilder,
        image_builder: &'a mut ImageRenderer,
    ) -> Self {
        let indication_tree = build_indication_tree_1(
            device,
            queue,
            store,
            screen_width,
            screen_height,
            selected_node,
            circle_builder,
            text_builder,
            image_builder,
        );
        Self { indication_tree }
    }
}
