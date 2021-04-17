use crate::arena::Structure;
use crate::arena::Value;
use crate::sphere::Sphere;
use crate::tree::Tree;
use crate::{arena::ArenaKey, render::text::TextConstraintBuilder};
use slotmap::new_key_type;
use slotmap::SlotMap;
use std::collections::vec_deque::VecDeque;

new_key_type! {
    pub struct SpatialTreeKey;
}

#[derive(Clone, Copy)]
pub struct SpatialTreeData {
    pub key: ArenaKey,
    pub sphere: Sphere,
}

// pub fn build_string_handler<'b, B: 'b, W: FnMut(&'b mut B, SpatialTreeData)>(
//     builder: &'b mut B,
//     with_instance: W,
// ) -> impl FnMut(&'b SlotMap<ArenaKey, Value>, SpatialTreeData) -> Vec<SpatialTreeData> {
//     |slot_map, spatial_tree_data| {
//         with_instance(builder, spatial_tree_data);
//         vec![]
//     }
// }

// build_string_handler(TextInstanceBuilder::with_instance)

pub fn build_spatial_tree(
    slot_map: &SlotMap<ArenaKey, Value>,
    start: ArenaKey,
    string_handler: &mut TextConstraintBuilder,
    image_handler: Handler,
    set_handler: Handler,
    map_handler: Handler,
) -> (Tree<SpatialTreeKey, SpatialTreeData>, SpatialTreeKey) {
    let mut tree: Tree<SpatialTreeKey, SpatialTreeData> = Tree::new();
    let root = tree.insert_root(SpatialTreeData {
        key: start,
        sphere: Sphere {
            center: (0.0, 0.0, 0.0).into(),
            radius: 1.0,
        },
    });

    let mut todo: VecDeque<SpatialTreeKey> = vec![root].into_iter().collect();

    while let Some(spatial_tree_key) = todo.pop_front() {
        let spatial_tree_data = tree.get(spatial_tree_key).copied().unwrap();
        match slot_map
            .get(spatial_tree_data.key)
            .unwrap()
            .structure
            .as_ref()
        {
            Structure::String(_) => handle_string(string_handler, spatial_tree_data),
            Structure::Image(_) => image_handler(slot_map, spatial_tree_data),
            Structure::Set(_) => set_handler(slot_map, spatial_tree_data),
            Structure::Map(_) => map_handler(slot_map, spatial_tree_data),
        }
        .into_iter()
        .for_each(|child_data| {
            todo.push_back(tree.insert_child(spatial_tree_key, child_data));
        });
    }

    (tree, root)
}

fn handle_string(
    string_handler: &mut TextConstraintBuilder,
    spatial_tree_data: SpatialTreeData,
) -> Vec<SpatialTreeData> {
    string_handler.with_instance(spatial_tree_data);
    vec![]
}
