//! # Visual layout control
//!
//! Decides where to place things on screen and how big they can possibly be.
//!
//! For the sake of this module, the screen is a square of side-length two. The
//! origin (0,0) of the screen is in the center of the square. X values increase
//! from -1 to 1 moving along the horizontal axis from left to right. Y values
//! increase from -1 to 1 moving along the vertical axis from bottom to top. The
//! screen is, of course, actually measured in pixels, and need not be a square.
//! The transformation from easy-to-work-with-square to
//! difficult-to-work-with-pixel-rectangle is automatically done elsewhere.
//!
//! Each object displayed on screen has associated with it a [`Sphere`]. The
//! `radius` of the sphere denotes the maximum possible size that the object can
//! be. The `center` of the sphere denotes the location of the center of the
//! object. These object-sphere pairs are represented as [`SpatialTreeData`],
//!
//! Each [`SpatialTreeData`] is arranged in a [`SpatialTree`]. The root of this
//! tree contains the currently-selected object centered at `(0,0)` with a
//! radius of `1.0`. If the currently-selected object is a container (a set or
//! map), then its node has children representing the size and locations of the
//! objects within the container. If an object is not a container (a string or
//! image), then its node does not have any children.
//!
//! The same object may appear more than once on screen with possibly differing
//! positions and sizes each time. Each visual instance of an object has
//! associated with it a unique [`SpatialTreeData`]; if an object is to appear
//! five times on screen, then there will be exactly five [`SpatialTreeData`]s
//! arranged in the [`SpatialTree`] that refer to it.
//!
//! [rooted tree]: https://en.wikipedia.org/wiki/Tree_(graph_theory)#Rooted_tree

use crate::arena::Structure;
use crate::arena::Value;
use crate::circle::{Circle, CirclePositioner, Point};
use crate::forest::Forest;
use crate::render::circle::{CircleRenderer, MIN_RADIUS};
use crate::render::image::ImageRenderer;
use crate::sphere::Sphere;
use crate::{arena::ArenaKey, render::text::TextRenderer};
use slotmap::new_key_type;
use slotmap::SlotMap;
use std::collections::{vec_deque::VecDeque, HashMap, HashSet};

new_key_type! {
    /// Key to access [`SpatialTreeData`] in a [`SpatialTree`].
    ///
    /// See [the module-level documentation](crate::spatial_tree) for more
    /// information.
    pub struct SpatialTreeKey;
}

/// An object with a center position and maximum possible size.
///
/// See [the module-level documentation](crate::spatial_tree) for more
/// information.
#[derive(Clone, Copy)]
pub struct SpatialTreeData {
    /// The object that will be displayed on screen.
    pub key: ArenaKey,
    /// The bounding sphere of the object. The sphere's `center` is the center
    /// of the object on screen. The sphere's `radius` is the maximum possible
    /// size of the object.
    pub sphere: Sphere,
}

/// A tree containing `SpatialTreeData`
///
/// See [the module-level documentation](crate::spatial_tree) for more
/// information.
pub struct SpatialTree {
    /// The single-tree [`Forest`] backing our tree.
    forest: Forest<SpatialTreeKey, SpatialTreeData>,
    /// The root node of the tree.
    root: SpatialTreeKey,
}

/// Creates, or regenerates, a spatial tree.
///
/// Removes an existing tree, if it exists, and generates a new one in its
/// place. Registers layout data with the appropriate renderers. The tree is
/// generated until we either run out of objects to layout, or the objects
/// become too small to be seen on screen.
///
/// Arguments:
///
/// * `forest`: Single-tree [`Forest`] possibly holding an existing tree.
/// * `existing_tree_root`: The root of the existing tree to remove, if any.
/// * `slot_map`: Object storage.
/// * `start`: Object to place at the root of the tree.
/// * `{text,image,circle}_renderer`: Queues instances to be drawn later.
/// * `screen_{width,height}`: Size of screen in pixels. Used to determine of
/// objects are visible on screen.
fn rebuild_tree(
    forest: &mut Forest<SpatialTreeKey, SpatialTreeData>,
    existing_tree_root: Option<SpatialTreeKey>,
    slot_map: &SlotMap<ArenaKey, Value>,
    start: ArenaKey,
    text_renderer: &mut TextRenderer,
    image_renderer: &mut ImageRenderer,
    circle_renderer: &mut CircleRenderer,
    screen_width: f32,
    screen_height: f32,
) -> SpatialTreeKey {
    existing_tree_root.map(|root| forest.remove_root(root));

    // The first instance is always in the center of the screen and has radius
    // one.
    let root = forest.insert_root(SpatialTreeData {
        key: start,
        sphere: Sphere {
            center: (0.0, 0.0, 0.0).into(),
            radius: 1.0,
        },
    });

    // We search through the slot_map for objects by starting with the root,
    // then moving to its contained objects (if any), then their contained
    // objects, and so on. Each processing step pops a value from the queue (the
    // current object to arrange), and then pushes zero or more values to the
    // queue (the contained objects to be arranged in further processing steps).
    let mut todo: VecDeque<SpatialTreeKey> = vec![root].into_iter().collect();
    while let Some(spatial_tree_key) = todo.pop_front() {
        let spatial_tree_data = forest.get(spatial_tree_key).copied().unwrap();
        // Ensure that the object we want to arrange is actually visible on
        // screen. If it isn't, ignore this object and move on to the next loop
        // iteration.
        if spatial_tree_data
            .sphere
            .screen_radius(screen_width, screen_height)
            > 1.0
        {
            match &slot_map.get(spatial_tree_data.key).unwrap().structure {
                Structure::String(_) => handle_string(text_renderer, spatial_tree_data),
                Structure::Image(_) => handle_image(image_renderer, spatial_tree_data),
                Structure::Set(set) => handle_set(circle_renderer, spatial_tree_data, set.as_ref()),
                Structure::Map(map) => handle_map(circle_renderer, spatial_tree_data, map.as_ref()),
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
    /// Removes the existing tree and generates a new one.
    ///
    /// See the documentation of [`rebuild_tree`] for
    /// more information.
    pub fn rebuild(
        &mut self,
        slot_map: &SlotMap<ArenaKey, Value>,
        start: ArenaKey,
        string_handler: &mut TextRenderer,
        image_handler: &mut ImageRenderer,
        circle_handler: &mut CircleRenderer,
        screen_width: f32,
        screen_height: f32,
    ) {
        self.root = rebuild_tree(
            &mut self.forest,
            Some(self.root),
            slot_map,
            start,
            string_handler,
            image_handler,
            circle_handler,
            screen_width,
            screen_height,
        );
    }

    /// Generates a new spatial tree.
    ///
    /// See the documentation of [`rebuild_tree`] for more information.
    pub fn new(
        slot_map: &SlotMap<ArenaKey, Value>,
        start: ArenaKey,
        string_handler: &mut TextRenderer,
        image_handler: &mut ImageRenderer,
        circle_handler: &mut CircleRenderer,
        screen_width: f32,
        screen_height: f32,
    ) -> Self {
        let mut forest: Forest<SpatialTreeKey, SpatialTreeData> = Forest::new();
        let root = rebuild_tree(
            &mut forest,
            None,
            slot_map,
            start,
            string_handler,
            image_handler,
            circle_handler,
            screen_width,
            screen_height,
        );
        SpatialTree { forest, root }
    }

    /// Returns the object at coordinates (`mouse_x`, `mouse_y`) on screen, if
    /// there is such an object.
    ///
    /// Only children of the root node are considered. A click on a
    /// child-of-child of the root node returns the the child, not the
    /// child-of-child.
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
            .children(self.root)
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

/// Maps screen coordinates to spatial tree coordinates.
///
/// There is currently a bug with this function that probably has something to
/// do with MIN_RADIUS; clicking near the inside circumference of a circle
/// doesn't always register a click on the circle.
///
/// This could probably be implemented better. It should probably create or have
/// passed in a transformation matrix and then invert it. See the camera struct
/// for more information.
///
/// If the camera is not looking straight-on, this function will give confusing
/// results. This should probably be fixed.
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

/// Lays out a string.
fn handle_string(
    string_handler: &mut TextRenderer,
    spatial_tree_data: SpatialTreeData,
) -> Vec<SpatialTreeData> {
    string_handler.with_instance(spatial_tree_data);
    vec![]
}

/// Lays out an image.
fn handle_image(
    image_handler: &mut ImageRenderer,
    spatial_tree_data: SpatialTreeData,
) -> Vec<SpatialTreeData> {
    image_handler.with_image(spatial_tree_data);
    vec![]
}

/// Lays out a set.
///
/// A single circle is registered to enclose the set. Each element of the set is
/// positioned along the inside circumference of the circle. The math for laying
/// out the elements of the set is handled by a [`CirclePositioner`].
///
/// The return value is a vector containing the layout information for the
/// elements of the set.
fn handle_set(
    circle_handler: &mut CircleRenderer,
    spatial_tree_data: SpatialTreeData,
    set: &HashSet<ArenaKey>,
) -> Vec<SpatialTreeData> {
    // The circle that encloses the set
    circle_handler.with_instance(spatial_tree_data.sphere);

    let sphere = if set.len() == 1 {
        // In the case where our set only contains one element, it is confusing
        // if that element were to be displayed the same size as the enclosing
        // circle. For instance, if that element was a set, we would not be able
        // to visually differentiate between a set and a
        // set-containing-a-single-set. For this reason, we make the
        // single-element a bit smaller than it would naturally be.
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
        .map(|(circle, key)| {
            let Circle { center, radius } = circle;
            let Point { x, y } = center;
            let radius = radius as f32;
            let other_sphere = Sphere {
                center: cgmath::vec3(x as f32, y as f32, 0.0),
                radius,
            };
            SpatialTreeData {
                sphere: other_sphere,
                key: *key,
            }
        })
        .collect()
}

/// Lays out a map.
///
/// A single circle is registered to enclose the map. The key-value pairs are
/// rendered as if the map was a set containing one two-element-set for each
/// key-value pair.
///
/// This should probably be improved to show visually which value is the key and
/// which value is the, well, value.
fn handle_map(
    circle_handler: &mut CircleRenderer,
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
