use std::collections::{hash_map, HashMap};
use std::hash::Hash;

use petgraph::graph::{EdgeIndex, NodeIndex};

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

pub enum Value {
    Association(Association),
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
