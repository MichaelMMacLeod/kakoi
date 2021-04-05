use std::collections::{hash_map, HashMap};
use std::hash::Hash;

use petgraph::graph::{EdgeIndex, NodeIndex};

use crate::flat_graph;

pub struct Indication {
    pub target: Key,
    pub route: EdgeIndex<u32>,
}

pub struct Association {
    pub indications: Vec<Indication>,
    pub focused_indication: usize,
    zoom: u32,
}

impl Association {
    pub fn new(indications: Vec<Indication>, focused_indication: usize, zoom: u32) -> Self {
        Self {
            indications,
            focused_indication,
            zoom,
        }
    }
    pub fn zoom(&self) -> f32 {
        const PERCENT_MULTIPLIER: f32 = 1.0 / u32::MAX as f32;
        self.zoom as f32 * PERCENT_MULTIPLIER
    }
    pub fn to_zoom(percent: f32) -> u32 {
        (percent.clamp(0.0, 1.0) * u32::MAX as f32) as u32
    }
}

pub struct Overlay {
    focus: Indication,
    message: Indication,
    message_visible: bool,
}

impl Overlay {
    pub fn new(
        store: &mut Store,
        flat_graph: &mut flat_graph::FlatGraph,
        focus: Key,
        message: Key,
    ) -> Key {
        let mut almost_self = Self {
            focus: Indication {
                target: focus,
                route: EdgeIndex::from(0),
            },
            message: Indication {
                target: message,
                route: EdgeIndex::from(0),
            },
            message_visible: true,
        };
        let self_node_index = flat_graph.g.add_node(flat_graph::Node);
        almost_self.focus.route =
            flat_graph
                .g
                .add_edge(self_node_index, NodeIndex::from(focus), flat_graph::Edge);
        almost_self.message.route =
            flat_graph
                .g
                .add_edge(self_node_index, NodeIndex::from(message), flat_graph::Edge);
        let self_key = Key(self_node_index);
        store.entry(self_key).or_insert(Value::Overlay(almost_self));
        self_key
    }

    pub fn focus(&self) -> &Key {
        &self.focus.target
    }

    pub fn message(&self) -> &Key {
        &self.message.target
    }

    pub fn message_visible(&self) -> bool {
        self.message_visible
    }

    pub fn set_focus(
        &mut self,
        flat_graph: &mut flat_graph::FlatGraph,
        self_key: Key,
        new_focus: Key,
    ) {
        flat_graph.g.remove_edge(self.focus.route);
        let new_route = flat_graph.g.add_edge(
            NodeIndex::from(self_key),
            NodeIndex::from(new_focus),
            flat_graph::Edge,
        );
        self.focus.target = new_focus;
        self.focus.route = new_route;
    }

    pub fn toggle_message_visibility(&mut self) {
        self.message_visible = !self.message_visible;
    }
}

// pub struct Map {
//     map: HashMap<Key, Key>,
// }

pub enum Value {
    Association(Association),
    Overlay(Overlay),
    // Map(Map),
    String(String),
    Image(image::RgbaImage),
}

impl Value {
    pub fn string(&self) -> Option<&String> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn image(&self) -> Option<&image::RgbaImage> {
        match self {
            Value::Image(i) => Some(i),
            _ => None,
        }
    }

    pub fn association(&self) -> Option<&Association> {
        match self {
            Value::Association(a) => Some(a),
            _ => None,
        }
    }

    pub fn association_mut(&mut self) -> Option<&mut Association> {
        match self {
            Value::Association(a) => Some(a),
            _ => None,
        }
    }

    pub fn overlay(&self) -> Option<&Overlay> {
        match self {
            Value::Overlay(o) => Some(o),
            _ => None,
        }
    }

    pub fn overlay_mut(&mut self) -> Option<&mut Overlay> {
        match self {
            Value::Overlay(o) => Some(o),
            _ => None,
        }
    }

    // pub fn map(&self) -> Option<&Map> {
    //     match self {
    //         Value::Map(m) => Some(m),
    //         _ => None,
    //     }
    // }

    // pub fn map_mut(&mut self) -> Option<&mut Map> {
    //     match self {
    //         Value::Map(m) => Some(m),
    //         _ => None,
    //     }
    // }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Key(NodeIndex<u32>);

impl From<NodeIndex<u32>> for Key {
    fn from(node_index: NodeIndex<u32>) -> Self {
        Self(node_index)
    }
}

impl From<Key> for NodeIndex<u32> {
    fn from(key: Key) -> Self {
        key.0
    }
}

pub struct Store {
    map: HashMap<Key, Value>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.map.get(&key)
    }

    pub fn entry(&mut self, key: Key) -> hash_map::Entry<Key, Value> {
        self.map.entry(key)
    }
}
