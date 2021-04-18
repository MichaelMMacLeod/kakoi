use crate::arena::Structure;
use crate::arena::Value;
use crate::circle::{Circle, CirclePositioner, Point};
use crate::forest::Forest;
use crate::render::circle::{CircleConstraintBuilder, MIN_RADIUS};
use crate::render::image::ImageRenderer;
use crate::sphere::Sphere;
use crate::{arena::ArenaKey, render::text::TextConstraintBuilder};
use slotmap::new_key_type;
use slotmap::SlotMap;
use std::collections::{vec_deque::VecDeque, HashMap, HashSet};

new_key_type! {
    pub struct SpatialTreeKey;
}

#[derive(Clone, Copy)]
pub struct SpatialTreeData {
    pub key: ArenaKey,
    pub sphere: Sphere,
}

pub struct SpatialTree {
    forest: Forest<SpatialTreeKey, SpatialTreeData>,
    root: Option<SpatialTreeKey>,
}

fn build(
    forest: &mut Forest<SpatialTreeKey, SpatialTreeData>,
    root: Option<SpatialTreeKey>,
    slot_map: &SlotMap<ArenaKey, Value>,
    start: ArenaKey,
    string_handler: &mut TextConstraintBuilder,
    image_handler: &mut ImageRenderer,
    circle_handler: &mut CircleConstraintBuilder,
    screen_width: f32,
    screen_height: f32,
) -> SpatialTreeKey {
    root.map(|root| forest.remove_root(root));

    let root = forest.insert_root(SpatialTreeData {
        key: start,
        sphere: Sphere {
            center: (0.0, 0.0, 0.0).into(),
            radius: 1.0,
        },
    });

    let mut todo: VecDeque<SpatialTreeKey> = vec![root].into_iter().collect();

    while let Some(spatial_tree_key) = todo.pop_front() {
        let spatial_tree_data = forest.get(spatial_tree_key).copied().unwrap();
        if spatial_tree_data
            .sphere
            .screen_radius(screen_width, screen_height)
            > 1.0
        {
            match slot_map
                .get(spatial_tree_data.key)
                .unwrap()
                .structure
                .as_ref()
            {
                Structure::String(_) => handle_string(string_handler, spatial_tree_data),
                Structure::Image(_) => handle_image(image_handler, spatial_tree_data),
                Structure::Set(set) => handle_set(circle_handler, spatial_tree_data, set),
                Structure::Map(map) => handle_map(circle_handler, spatial_tree_data, map),
            }
            .into_iter()
            .for_each(|child_data| {
                todo.push_back(forest.insert_child(spatial_tree_key, child_data));
            });
        }
    }

    root
}

impl SpatialTree {
    pub fn rebuild(
        &mut self,
        slot_map: &SlotMap<ArenaKey, Value>,
        start: ArenaKey,
        string_handler: &mut TextConstraintBuilder,
        image_handler: &mut ImageRenderer,
        circle_handler: &mut CircleConstraintBuilder,
        screen_width: f32,
        screen_height: f32,
    ) {
        self.root = Some(build(
            &mut self.forest,
            self.root,
            slot_map,
            start,
            string_handler,
            image_handler,
            circle_handler,
            screen_width,
            screen_height,
        ));
    }

    pub fn new(
        slot_map: &SlotMap<ArenaKey, Value>,
        start: ArenaKey,
        string_handler: &mut TextConstraintBuilder,
        image_handler: &mut ImageRenderer,
        circle_handler: &mut CircleConstraintBuilder,
        screen_width: f32,
        screen_height: f32,
    ) -> Self {
        let mut forest: Forest<SpatialTreeKey, SpatialTreeData> = Forest::new();
        let root = Some(build(
            &mut forest,
            None,
            slot_map,
            start,
            string_handler,
            image_handler,
            circle_handler,
            screen_width,
            screen_height,
        ));
        SpatialTree { forest, root }
    }

    pub fn click(
        &self,
        screen_width: f32,
        screen_height: f32,
        mouse_x: f32,
        mouse_y: f32,
    ) -> Option<ArenaKey> {
        let (mouse_x, mouse_y) =
            screen_to_view_coordinates(mouse_x, mouse_y, screen_width, screen_height);
        self.forest
            .children(self.root?)
            .unwrap()
            .iter()
            .copied()
            .find_map(|child| {
                let SpatialTreeData { key, sphere } = self.forest.get(child).unwrap();
                let dx = sphere.center.x - mouse_x;
                let dy = sphere.center.y - mouse_y;
                let inside_rad = (dx * dx + dy * dy).sqrt() <= sphere.radius;
                if inside_rad {
                    Some(*key)
                } else {
                    None
                }
            })
    }
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

fn handle_string(
    string_handler: &mut TextConstraintBuilder,
    spatial_tree_data: SpatialTreeData,
) -> Vec<SpatialTreeData> {
    string_handler.with_instance(spatial_tree_data);
    vec![]
}

fn handle_image(
    image_handler: &mut ImageRenderer,
    spatial_tree_data: SpatialTreeData,
) -> Vec<SpatialTreeData> {
    image_handler.with_image(spatial_tree_data);
    vec![]
}

fn handle_set(
    circle_handler: &mut CircleConstraintBuilder,
    spatial_tree_data: SpatialTreeData,
    set: &HashSet<ArenaKey>,
) -> Vec<SpatialTreeData> {
    circle_handler.with_instance(spatial_tree_data.sphere);
    let sphere = if set.len() == 1 {
        Sphere {
            center: spatial_tree_data.sphere.center,
            radius: spatial_tree_data.sphere.radius * MIN_RADIUS,
        }
    } else {
        spatial_tree_data.sphere
    };
    let circle_positioner = CirclePositioner::new(
        (sphere.radius * MIN_RADIUS) as f64,
        set.len() as u64,
        0.0,
        Point {
            x: sphere.center.x as f64,
            y: sphere.center.y as f64,
        },
        0.0,
    );
    circle_positioner
        .into_iter()
        .zip(set.iter())
        .filter_map(|(circle, key)| {
            let Circle { center, radius } = circle;
            let Point { x, y } = center;
            let radius = radius as f32;
            let other_sphere = Sphere {
                center: cgmath::vec3(x as f32, y as f32, 0.0),
                radius,
            };
            Some(SpatialTreeData {
                sphere: other_sphere,
                key: *key,
            })
        })
        .collect()
}

fn handle_map(
    circle_handler: &mut CircleConstraintBuilder,
    spatial_tree_data: SpatialTreeData,
    map: &HashMap<ArenaKey, ArenaKey>,
) -> Vec<SpatialTreeData> {
    circle_handler.with_instance(spatial_tree_data.sphere);
    let sphere = if map.len() == 1 {
        Sphere {
            center: spatial_tree_data.sphere.center,
            radius: spatial_tree_data.sphere.radius * MIN_RADIUS,
        }
    } else {
        spatial_tree_data.sphere
    };
    let circle_positioner = CirclePositioner::new(
        (sphere.radius * MIN_RADIUS) as f64,
        map.len() as u64,
        0.0,
        Point {
            x: sphere.center.x as f64,
            y: sphere.center.y as f64,
        },
        0.0,
    );
    circle_positioner
        .into_iter()
        .zip(map.iter())
        .map(|(circle, (key, value))| {
            let Circle { center, radius } = circle;
            let Point { x, y } = center;
            let radius = radius as f32;
            let other_sphere = Sphere {
                center: cgmath::vec3(x as f32, y as f32, 0.0),
                radius,
            };
            circle_handler.with_instance(other_sphere);
            let sub_circle_positioner =
                CirclePositioner::new(circle.radius, 2, 0.0, circle.center, 0.0);
            sub_circle_positioner
                .into_iter()
                .zip(vec![key, value])
                .map(|(circle, &key)| {
                    let Circle { center, radius } = circle;
                    let Point { x, y } = center;
                    let radius = radius as f32;
                    let other_sphere = Sphere {
                        center: cgmath::vec3(x as f32, y as f32, 0.0),
                        radius,
                    };
                    // circle_handler.with_instance(other_sphere);
                    SpatialTreeData {
                        sphere: other_sphere,
                        key,
                    }
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect()
}
