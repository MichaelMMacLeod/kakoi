use slotmap::{new_key_type, SlotMap};
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
};

new_key_type! {
    pub struct ArenaKey;
}

#[derive(PartialEq, Eq, Hash)]
pub enum MapRoute {
    Key,
    ValueOf(ArenaKey),
}

#[derive(PartialEq, Eq, Hash)]
pub enum Route {
    Set,
    Map(MapRoute),
}

pub enum Structure {
    Set(HashSet<ArenaKey>),
    // Overlay(Overlay),
    Map(HashMap<ArenaKey, ArenaKey>),
    Image(image::RgbaImage),
    String(String),
}

pub struct Value {
    pub structure: Box<Structure>,
    pub inclusions: HashSet<(ArenaKey, Route)>,
}

pub struct Arena {
    pub slot_map: SlotMap<ArenaKey, Value>,
    pub register_map: ArenaKey,
    selected_register: ArenaKey,
    lookup_map: HashMap<u64, ArenaKey>,
}

fn get_hash_of<V: Hash>(value: V) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn insert_string<S: Into<String>>(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    lookup_map: &mut HashMap<u64, ArenaKey>,
    string: S,
) -> ArenaKey {
    let string = string.into();

    let mut hasher = DefaultHasher::new();
    string.hash(&mut hasher);
    let hash = hasher.finish();
    match lookup_map.get(&hash).copied() {
        // if we previously inserted the string, use that key instead
        Some(key) => key,
        // otherwise, insert the string into the slot map and the lookup map
        None => {
            let key = slot_map.insert(Value {
                structure: Box::new(Structure::String(string)),
                inclusions: HashSet::new(),
            });
            lookup_map.entry(hash).or_insert(key);
            key
        }
    }
}

fn insert_image(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    lookup_map: &mut HashMap<u64, ArenaKey>,
    image: image::RgbaImage,
) -> ArenaKey {
    let mut hasher = DefaultHasher::new();
    image.hash(&mut hasher);
    let hash = hasher.finish();
    match lookup_map.get(&hash).copied() {
        // if we previously inserted the image, use that key instead
        Some(key) => key,
        // otherwise, insert the image into the slot map and the lookup map
        None => {
            let key = slot_map.insert(Value {
                structure: Box::new(Structure::Image(image)),
                inclusions: HashSet::new(),
            });
            lookup_map.entry(hash).or_insert(key);
            key
        }
    }
}

fn add_inclusion(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    indicated: ArenaKey,
    indicator: ArenaKey,
    route: Route,
) {
    slot_map
        .get_mut(indicated)
        .unwrap()
        .inclusions
        .insert((indicator, route));
}

fn remove_inclusion(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    indicated: ArenaKey,
    indicator: ArenaKey,
    route: Route,
) {
    slot_map
        .get_mut(indicated)
        .unwrap()
        .inclusions
        .remove(&(indicator, route));
}

fn insert_set(slot_map: &mut SlotMap<ArenaKey, Value>, set: HashSet<ArenaKey>) -> ArenaKey {
    let indications = set.iter().copied().collect::<Vec<_>>();

    // insert the set into the slot map
    let key = slot_map.insert(Value {
        structure: Box::new(Structure::Set(set)),
        inclusions: HashSet::new(),
    });

    // add the set's key to the inclusions of each value in the set
    for k in indications {
        add_inclusion(slot_map, k, key, Route::Set);
    }

    key
}

fn insert_map(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    map: HashMap<ArenaKey, ArenaKey>,
) -> ArenaKey {
    let indications = map.iter().map(|(&k, &v)| (k, v)).collect::<Vec<_>>();

    // insert the map into the slot map
    let key = slot_map.insert(Value {
        structure: Box::new(Structure::Map(map)),
        inclusions: HashSet::new(),
    });

    // add the map's key to the inclusions of each key and value in the map
    for (k, v) in indications {
        add_inclusion(slot_map, k, key, Route::Map(MapRoute::Key));
        add_inclusion(slot_map, v, key, Route::Map(MapRoute::ValueOf(k)));
    }

    key
}

fn insert_structure(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    lookup_map: &mut HashMap<u64, ArenaKey>,
    structure: Structure,
) -> ArenaKey {
    match structure {
        Structure::String(string) => insert_string(slot_map, lookup_map, string),
        Structure::Image(image) => insert_image(slot_map, lookup_map, image),
        Structure::Set(set) => insert_set(slot_map, set),
        Structure::Map(map) => insert_map(slot_map, map),
    }
}

fn set_insert(slot_map: &mut SlotMap<ArenaKey, Value>, set: ArenaKey, value: ArenaKey) {
    add_inclusion(slot_map, value, set, Route::Set);
    match slot_map.get_mut(set).unwrap().structure.as_mut() {
        Structure::Set(hash_set) => {
            hash_set.insert(value);
        }
        _ => panic!(),
    }
}

fn set_remove(slot_map: &mut SlotMap<ArenaKey, Value>, set: ArenaKey, value: ArenaKey) {
    remove_inclusion(slot_map, value, set, Route::Set);
    match slot_map.get_mut(set).unwrap().structure.as_mut() {
        Structure::Set(hash_set) => {
            hash_set.remove(&value);
        }
        _ => panic!(),
    }
}

fn set_union(slot_map: &mut SlotMap<ArenaKey, Value>, set_to_modify: ArenaKey, other: ArenaKey) {
    // add `set_to_modify` to the inclusions of the indications of `other`
    let other_indications = match slot_map.get(other).unwrap().structure.as_ref() {
        Structure::Set(hash_set) => hash_set.iter().copied().collect::<Vec<_>>(),
        _ => panic!(),
    };
    for k in other_indications {
        add_inclusion(slot_map, k, set_to_modify, Route::Set);
    }

    // Add the indications of `other` to the indications of `set_to_modify`
    match slot_map.get_disjoint_mut([set_to_modify, other]).unwrap() {
        [Value {
            structure: set_to_modify,
            ..
        }, Value {
            structure: other, ..
        }] => match [set_to_modify.as_mut(), other.as_mut()] {
            [Structure::Set(set_to_modify), Structure::Set(other)] => {
                for i in other.iter().copied() {
                    set_to_modify.insert(i);
                }
            }
            _ => panic!(),
        },
    }
}

fn set_difference(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    set_to_modify: ArenaKey,
    other: ArenaKey,
) {
    // remove `set_to_modify` from the inclusions of the indications of `other`.
    let other_indications = match slot_map.get(other).unwrap().structure.as_ref() {
        Structure::Set(hash_set) => hash_set.iter().copied().collect::<Vec<_>>(),
        _ => panic!(),
    };
    for k in other_indications {
        remove_inclusion(slot_map, k, set_to_modify, Route::Set);
    }

    // Remove the indications of `other` from the indications of `set_to_modify`.
    match slot_map.get_disjoint_mut([set_to_modify, other]).unwrap() {
        [Value {
            structure: set_to_modify,
            ..
        }, Value {
            structure: other, ..
        }] => match [set_to_modify.as_mut(), other.as_mut()] {
            [Structure::Set(set_to_modify), Structure::Set(other)] => {
                for i in other.iter() {
                    set_to_modify.remove(i);
                }
            }
            _ => panic!(),
        },
    }
}

// If there was an old value associated with `key`, remove `map` from its inclusions.
fn map_remove_value_inclusion(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    map: ArenaKey,
    key: ArenaKey,
) {
    match slot_map.get(map).unwrap().structure.as_ref() {
        Structure::Map(hash_map) => {
            hash_map.get(&key).copied().map(|old_value| {
                remove_inclusion(slot_map, old_value, map, Route::Map(MapRoute::ValueOf(key)));
            });
        }
        _ => panic!(),
    }
}

fn map_remove(slot_map: &mut SlotMap<ArenaKey, Value>, map: ArenaKey, key: ArenaKey) {
    map_remove_value_inclusion(slot_map, map, key);
    remove_inclusion(slot_map, key, map, Route::Map(MapRoute::Key));
    match slot_map.get_mut(map).unwrap().structure.as_mut() {
        Structure::Map(hash_map) => {
            hash_map.remove(&key);
        }
        _ => panic!(),
    }
}

fn map_insert(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    map: ArenaKey,
    key: ArenaKey,
    value: ArenaKey,
) {
    // If there was an old value associated with `key`, remove `map` from its inclusions.
    map_remove_value_inclusion(slot_map, map, key);

    // Add `hash` to the inclusions of `value` and `key`
    add_inclusion(slot_map, value, map, Route::Map(MapRoute::ValueOf(key)));
    add_inclusion(slot_map, key, map, Route::Map(MapRoute::Key));

    // Add (key, value) to the map
    match slot_map.get_mut(map).unwrap().structure.as_mut() {
        Structure::Map(hash_map) => {
            hash_map.insert(key, value);
        }
        _ => panic!(),
    }
}

fn map_get(slot_map: &SlotMap<ArenaKey, Value>, map: ArenaKey, key: ArenaKey) -> Option<ArenaKey> {
    match slot_map.get(map).unwrap().structure.as_ref() {
        Structure::Map(hash_map) => hash_map.get(&key).copied(),
        _ => panic!(),
    }
}

impl Arena {
    pub fn new() -> Self {
        let mut slot_map = SlotMap::with_key();
        let mut lookup_map = HashMap::new();
        let selected_register = insert_string(&mut slot_map, &mut lookup_map, ".");
        let empty_set = insert_set(&mut slot_map, HashSet::new());
        let register_map = insert_map(
            &mut slot_map,
            vec![(selected_register, empty_set)].into_iter().collect(),
        );
        Self {
            slot_map,
            register_map,
            selected_register,
            lookup_map,
        }
    }

    pub fn register<S: Into<String>>(&mut self, register: S) -> Option<ArenaKey> {
        let register = insert_string(&mut self.slot_map, &mut self.lookup_map, register.into());
        map_get(&self.slot_map, self.register_map, register)
    }

    pub fn bind_register<S: Into<String>>(&mut self, register: S, value: ArenaKey) {
        let register = insert_string(&mut self.slot_map, &mut self.lookup_map, register.into());
        map_insert(&mut self.slot_map, self.register_map, register, value);
    }

    pub fn bind_register_to_empty_set<S: Into<String>>(&mut self, register: S) {
        let register = insert_string(&mut self.slot_map, &mut self.lookup_map, register.into());
        let set = insert_set(&mut self.slot_map, HashSet::new());
        map_insert(&mut self.slot_map, self.register_map, register, set);
    }

    pub fn bind_register_to_string<S: Into<String>>(&mut self, register: S, string: S) {
        let register = insert_string(&mut self.slot_map, &mut self.lookup_map, register.into());
        let string = insert_string(&mut self.slot_map, &mut self.lookup_map, string.into());
        map_insert(&mut self.slot_map, self.register_map, register, string);
    }

    pub fn bind_register_to_register_value<S: Into<String>>(
        &mut self,
        to_be_binded: S,
        to_lookup: S,
    ) {
        let to_be_binded = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            to_be_binded.into(),
        );
        let to_lookup = insert_string(&mut self.slot_map, &mut self.lookup_map, to_lookup.into());
        map_get(&self.slot_map, self.register_map, to_lookup).map(|k| {
            map_insert(&mut self.slot_map, self.register_map, to_be_binded, k);
        });
    }

    pub fn set_insert_string<S: Into<String>>(&mut self, set_register: S, string: S) -> Option<()> {
        let set_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            set_register.into(),
        );
        let string = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            string.into(),
        );

        let set = map_get(&self.slot_map, self.register_map, set_register)?;

        set_insert(&mut self.slot_map, set, string);

        Some(())
    }

    pub fn set_insert<S: Into<String>>(
        &mut self,
        set_register: S,
        insertion_register: S,
    ) -> Option<()> {
        let set_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            set_register.into(),
        );
        let insertion_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            insertion_register.into(),
        );

        let set = map_get(&self.slot_map, self.register_map, set_register)?;
        let insertion = map_get(&self.slot_map, self.register_map, insertion_register)?;

        set_insert(&mut self.slot_map, set, insertion);

        Some(())
    }

    pub fn set_remove<S: Into<String>>(
        &mut self,
        set_register: S,
        removal_register: S,
    ) -> Option<()> {
        let set_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            set_register.into(),
        );
        let removal_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            removal_register.into(),
        );

        let set = map_get(&self.slot_map, self.register_map, set_register)?;
        let removal = map_get(&self.slot_map, self.register_map, removal_register)?;

        set_remove(&mut self.slot_map, set, removal);

        Some(())
    }

    pub fn set_union<S: Into<String>>(
        &mut self,
        set_modified_register: S,
        set_other_register: S,
    ) -> Option<()> {
        let set_modified_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            set_modified_register.into(),
        );
        let set_other_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            set_other_register.into(),
        );

        let set_modified = map_get(&self.slot_map, self.register_map, set_modified_register)?;
        let set_other = map_get(&self.slot_map, self.register_map, set_other_register)?;

        set_union(&mut self.slot_map, set_modified, set_other);

        Some(())
    }

    pub fn set_difference<S: Into<String>>(
        &mut self,
        set_modified_register: S,
        set_other_register: S,
    ) -> Option<()> {
        let set_modified_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            set_modified_register.into(),
        );
        let set_other_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            set_other_register.into(),
        );

        let set_modified = map_get(&self.slot_map, self.register_map, set_modified_register)?;
        let set_other = map_get(&self.slot_map, self.register_map, set_other_register)?;

        set_difference(&mut self.slot_map, set_modified, set_other);

        Some(())
    }
}

// pub fn naming_example() -> (Self, ArenaKey) {
//     let mut arena = Arena::new();

//     let consonant_set = {
//         let consonants = [
//             "b", "c", "d", "f", "g", "h", "j", "k", "l", "m", "n", "p", "q", "r", "s", "t",
//             "v", "w", "x", "y", "z",
//         ]
//         .iter()
//         .map(|&s| arena.insert(Structure::String(s.into())).unwrap())
//         .collect();
//         arena.insert(Structure::Set(consonants)).unwrap()
//     };

//     let vowel_set = {
//         let vowels = ["a", "e", "i", "o", "u"]
//             .iter()
//             .map(|&s| arena.insert(Structure::String(s.into())).unwrap())
//             .collect();
//         arena.insert(Structure::Set(vowels)).unwrap()
//     };

//     let kakoi_set = {
//         let kakoi_example_1 = {
//             let kakoi_example_1 =
//                 include_bytes!("resources/images/Kakoi Example 1 [senseis.xmp.net].png");
//             image::load_from_memory(kakoi_example_1)
//                 .unwrap()
//                 .into_rgba8()
//         };
//         let kakoi_example_2 = {
//             let kakoi_example_2 =
//                 include_bytes!("resources/images/Kakoi Example 2 [senseis.xmp.net].png");
//             image::load_from_memory(kakoi_example_2)
//                 .unwrap()
//                 .into_rgba8()
//         };
//         let kakoi_example_3 = {
//             let kakoi_example_3 =
//                 include_bytes!("resources/images/Kakoi Example 1 [senseis.xmp.net] wide.png");
//             image::load_from_memory(kakoi_example_3)
//                 .unwrap()
//                 .into_rgba8()
//         };
//         let kakoi = vec![kakoi_example_1, kakoi_example_2, kakoi_example_3]
//             .drain(..)
//             .map(|s| arena.insert(Structure::Image(s)).unwrap())
//             .collect();
//         arena.insert(Structure::Set(kakoi)).unwrap()
//     };
// }
