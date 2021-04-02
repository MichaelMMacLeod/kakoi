use std::collections::VecDeque;

use petgraph::graph::NodeIndex;

use crate::{circle::Point, sphere::Sphere, store};
use crate::{
    circle::{Circle, CirclePositioner},
    flat_graph::{self, FlatGraph},
};

use super::{
    circle::{CircleConstraintBuilder, MIN_RADIUS},
    indication_tree::{self, Tree, TreeNode},
    text::TextConstraintBuilder,
};

pub struct Builder {
    pub indication_tree: Tree,
}

fn indications_of<'a>(
    flat_graph: &'a FlatGraph,
    index: NodeIndex<u32>,
    num_indications: usize,
) -> Vec<(u32, NodeIndex<u32>)> {
    let mut walker = flat_graph
        .g
        .neighbors_directed(index, petgraph::Direction::Outgoing)
        .detach();

    let mut indications = Vec::with_capacity(num_indications);

    while let Some((edge, node)) = walker.next(&flat_graph.g) {
        let flat_graph::Edge(n) = flat_graph.g[edge];
        indications.push((n, node));
    }

    indications
}

fn build_indication_tree_2<'a>(
    store: &'a store::Store,
    flat_graph: &'a FlatGraph,
    tree_impl: &'a mut indication_tree::Impl,
    root_index: NodeIndex<u32>,
    aspect_ratio: f32,
    circle_builder: &'a mut CircleConstraintBuilder,
    text_builder: &'a mut TextConstraintBuilder,
) {
    let mut todo = VecDeque::new();
    todo.push_back(root_index);

    let unit_sphere = Sphere {
        center: cgmath::vec3(0.0, 0.0, 0.0),
        radius: 1.0,
    };

    while let Some(indication_tree_index) = todo.pop_front() {
        let TreeNode {
            sphere,
            flat_graph_index,
        } = &tree_impl[NodeIndex::from(indication_tree_index)];

        match &flat_graph.g[NodeIndex::from(*flat_graph_index)] {
            flat_graph::Node::Leaf(key) => match store.get(key).unwrap() {
                store::Value::String(_) => {
                    text_builder.with_instance(*sphere, *key);
                }
            },
            flat_graph::Node::Branch(flat_graph::Branch {
                num_indications,
                focused_indication,
                zoom,
            }) => {
                let num_indications = *num_indications;
                let zoom = *zoom;
                let focused_indication = *focused_indication;

                let focus_angle =
                    2.0 * std::f32::consts::PI / num_indications as f32 * focused_indication as f32;
                let circle_positioner = CirclePositioner::new(
                    (sphere.radius * MIN_RADIUS) as f64,
                    num_indications as u64,
                    zoom as f64,
                    Point {
                        x: sphere.center.x as f64,
                        y: sphere.center.y as f64,
                    },
                    focus_angle as f64,
                );

                let mut indications =
                    indications_of(flat_graph, *flat_graph_index, num_indications as usize);
                indications.sort_by_key(|(n, _)| *n);

                let (before_focused, focused_and_after): (Vec<_>, Vec<_>) = indications
                    .iter()
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

                        if unit_sphere.can_observe(&other_sphere, aspect_ratio) {
                            let indicated_index = tree_impl.add_node(indication_tree::TreeNode {
                                flat_graph_index: *node,
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
    store: &'a store::Store,
    flat_graph: &'a FlatGraph,
    aspect_ratio: f32,
    selected_node: NodeIndex<u32>,
    circle_builder: &'a mut CircleConstraintBuilder,
    text_builder: &'a mut TextConstraintBuilder,
) -> Tree {
    let mut tree_impl = indication_tree::Impl::new();

    let first_flat_graph_index = selected_node;
    let first_sphere = Sphere {
        center: cgmath::vec3(0.0, 0.0, 0.0),
        radius: 1.0,
    };
    let first_indication_tree_index = tree_impl
        .add_node(TreeNode {
            flat_graph_index: first_flat_graph_index,
            sphere: first_sphere,
        })
        .into();

    circle_builder.with_instance(first_sphere);

    let mut todo = VecDeque::new();
    todo.push_back(first_indication_tree_index);

    build_indication_tree_2(
        store,
        flat_graph,
        &mut tree_impl,
        first_indication_tree_index,
        aspect_ratio,
        circle_builder,
        text_builder,
    );

    Tree {
        g: tree_impl,
        root: first_indication_tree_index,
    }
}

impl Builder {
    pub fn new<'a>(
        store: &'a store::Store,
        flat_graph: &'a FlatGraph,
        aspect_ratio: f32,
        circle_builder: &'a mut CircleConstraintBuilder,
        text_builder: &'a mut TextConstraintBuilder,
    ) -> Self {
        Self::new_with_selection(
            store,
            flat_graph,
            aspect_ratio,
            flat_graph.focused.unwrap(),
            circle_builder,
            text_builder,
        )
    }

    pub fn new_with_selection<'a>(
        store: &'a store::Store,
        flat_graph: &'a FlatGraph,
        aspect_ratio: f32,
        selected_node: NodeIndex<u32>,
        circle_builder: &'a mut CircleConstraintBuilder,
        text_builder: &'a mut TextConstraintBuilder,
    ) -> Self {
        let indication_tree = build_indication_tree_1(
            store,
            flat_graph,
            aspect_ratio,
            selected_node,
            circle_builder,
            text_builder,
        );
        Self { indication_tree }
    }
}
