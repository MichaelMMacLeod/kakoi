use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

#[derive(Hash)]
pub enum Value {
    String(String),
}

impl Value {
    pub fn string(&self) -> Option<&String> {
        match self {
            Value::String(s) => Some(s),
            // _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Key(u64);

pub struct Store {
    map: HashMap<u64, Value>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.map.get(&key.0)
    }

    pub fn insert(&mut self, item: Value) -> Key {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        let key = hasher.finish();
        self.map.entry(key).or_insert(item);
        Key(key)
    }
}