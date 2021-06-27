//! Object storage
//!
//! Everything we can interact with in Kakoi is backed by a [`Value`] that is
//! stored in a single [`Arena`].

use slotmap::{new_key_type, SlotMap};
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
};

new_key_type! {
    /// Key for accessing [`Value`]s in an [`Arena`].
    pub struct ArenaKey;
}

/// Describes the way in which a containee [`Value`] is included inside of a
/// [`Structure::List`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ListRoute {
    index: usize,
}

/// Describes the way in which a containee [`Value`] is included inside of a
/// [`Structure::Map`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum MapRoute {
    /// For [`Value`]s contained within the key of a map.
    Key,
    /// For [`Value`]s contained within the value of a map associated with a
    /// specific key of the map.
    ValueOf(ArenaKey),
}

/// Describes the way in which a containee [`Value`] is included inside of a
/// container [`Value`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Route {
    /// For [`Value`]s contained within a [`Structure::Set`].
    Set,
    /// For [`Value`]s contained within a [`Structure::Map`].
    Map(MapRoute),
    /// For [`Value`]s contained within a [`Structure::List`].
    List(ListRoute),
}

/// Data container inside [`Value`].
///
/// The data associated with each variant is boxed to save memory. It may not be
/// necessary to box every variant. This has not been profiled.
#[derive(Debug)]
pub enum Structure {
    /// A [set] for associating unique objects.
    ///
    /// [set]: https://en.wikipedia.org/wiki/Set_(mathematics)
    Set(Box<HashSet<ArenaKey>>),
    // A list. Like a set, but has a specified ordering.
    List(Box<Vec<ArenaKey>>),
    /// A [map] for associating unique key-objects with value-objects. A single
    /// key may only be associated with one value, but there may be many keys
    /// associated with the same value.
    ///
    /// [map]: https://en.wikipedia.org/wiki/Associative_array
    Map(Box<HashMap<ArenaKey, ArenaKey>>),
    /// An image. Does not contain any other values.
    Image(Box<image::RgbaImage>),
    /// A string. Does not contain any other values.
    String(Box<String>),
}

/// Container that also tracks which [`Value`]s contain it.
#[derive(Debug)]
pub struct Value {
    /// The data associated with this object.
    pub structure: Structure,
    /// A set of objects that contain this value. Each [`Route`] describes how
    /// this value is contained.
    ///
    /// It is necessary to track the inclusions of each value in order to easily
    /// (computationally speaking) determine which values contain this value.
    /// Tracking the routes is necessary because a single value may be contained
    /// in the same container multiple times in different places. For instance,
    /// it could be both a key and a value in the same map.
    ///
    /// All functions that modify a [`Value`]'s `structure` should predictably
    /// modify the `inclusions` of contained values when necessary. For
    /// instance, if this value is a set and we are inserting a string into it,
    /// the string value's `inclusions` field must be modified to point to this
    /// value through [`Route::Set`].
    pub inclusions: HashSet<(ArenaKey, Route)>,
}

/// Storage container for [`Value`]s.
pub struct Arena {
    /// Underlying container implementation.
    pub slot_map: SlotMap<ArenaKey, Value>,
    /// Key that refers to a `Structure::Map` in the `slot_map` pairing
    /// registers with their values.
    pub register_map: ArenaKey,
    /// Associates the hash of the unboxed data inside of a [`Structure`]
    /// variant with the [`ArenaKey`] it is bound to in the `slot_map`. We use
    /// this to determine if hashable values have already been inserted into the
    /// [`Arena`].
    ///
    /// This is necessary when we need to find the key of a value in the
    /// `slot_map`. For instance, many functions on [`Arena`] take strings that
    /// represent register names. Without this field, there would be no easy way
    /// to know if we had already inserted the register string into the
    /// `register_map`, and we would be unable to easily look up its value.
    lookup_map: HashMap<u64, ArenaKey>,
}

/// Inserts a [`String`] into a [`SlotMap`].
///
/// If the string's hash already has an entry in the `lookup_map`, that key is
/// returned and the `slot_map` is not modified.
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
                structure: Structure::String(Box::new(string)),
                inclusions: HashSet::new(),
            });
            lookup_map.entry(hash).or_insert(key);
            key
        }
    }
}

/// Inserts an [`image`](image::RgbaImage) into a [`SlotMap`].
///
/// If the image's hash already has an entry in the `lookup_map`, that key is
/// returned and the `slot_map` is not modified.
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
                structure: Structure::Image(Box::new(image)),
                inclusions: HashSet::new(),
            });
            lookup_map.entry(hash).or_insert(key);
            key
        }
    }
}

/// Convenience function for marking a [`Value`] as being contained within
/// another.
///
/// * `indicated`: the contained value
/// * `indicator`: the container of `indicated`
/// * `route`: the way in which `indicated` is contained within `indicator`
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

/// Convenience function for marking a [`Value`] as not being contained within
/// another.
///
/// * `indicated`: the contained value
/// * `indicator`: the container of `indicated`
/// * `route`: the way in which `indicated` is contained within `indicator`
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

/// Inserts a [`set`](HashSet) into a [`SlotMap`].
fn insert_set(slot_map: &mut SlotMap<ArenaKey, Value>, set: HashSet<ArenaKey>) -> ArenaKey {
    let indications = set.iter().copied().collect::<Vec<_>>();

    // insert the set into the slot map
    let key = slot_map.insert(Value {
        structure: Structure::Set(Box::new(set)),
        inclusions: HashSet::new(),
    });

    // add the set's key to the inclusions of each value in the set
    for k in indications {
        add_inclusion(slot_map, k, key, Route::Set);
    }

    key
}

/// Inserts a [`list`](Vec) into a [`SlotMap`].
fn insert_list(slot_map: &mut SlotMap<ArenaKey, Value>, list: Vec<ArenaKey>) -> ArenaKey {
    let indications = list.clone();

    // insert the list into the slot map
    let key = slot_map.insert(Value {
        structure: Structure::List(Box::new(list)),
        inclusions: HashSet::new(),
    });

    // add the list's key to the inclusions of each value in the list
    for (n, k) in indications.into_iter().enumerate() {
        add_inclusion(slot_map, k, key, Route::List(ListRoute { index: n }));
    }

    key
}

/// Inserts a [`map`](HashMap) into a [`SlotMap`].
fn insert_map(
    slot_map: &mut SlotMap<ArenaKey, Value>,
    map: HashMap<ArenaKey, ArenaKey>,
) -> ArenaKey {
    let indications = map.iter().map(|(&k, &v)| (k, v)).collect::<Vec<_>>();

    // insert the map into the slot map
    let key = slot_map.insert(Value {
        structure: Structure::Map(Box::new(map)),
        inclusions: HashSet::new(),
    });

    // add the map's key to the inclusions of each key and value in the map
    for (k, v) in indications {
        add_inclusion(slot_map, k, key, Route::Map(MapRoute::Key));
        add_inclusion(slot_map, v, key, Route::Map(MapRoute::ValueOf(k)));
    }

    key
}

// fn insert_structure(
//     slot_map: &mut SlotMap<ArenaKey, Value>,
//     lookup_map: &mut HashMap<u64, ArenaKey>,
//     structure: Structure,
// ) -> ArenaKey {
//     match structure {
//         Structure::String(string) => insert_string(slot_map, lookup_map, string),
//         Structure::Image(image) => insert_image(slot_map, lookup_map, image),
//         Structure::Set(set) => insert_set(slot_map, set),
//         Structure::Map(map) => insert_map(slot_map, map),
//     }
// }

fn list_push(slot_map: &mut SlotMap<ArenaKey, Value>, list: ArenaKey, value: ArenaKey) {
    let index = match &mut slot_map.get_mut(list).unwrap().structure {
        Structure::List(vec) => {
            let index = vec.len();
            vec.push(value);
            index
        }
        _ => panic!(),
    };

    add_inclusion(slot_map, value, list, Route::List(ListRoute { index }));
}

fn list_pop(slot_map: &mut SlotMap<ArenaKey, Value>, list: ArenaKey) {
    let (index, value) = match &mut slot_map.get_mut(list).unwrap().structure {
        Structure::List(vec) => {
            let value = vec.pop();
            (vec.len(), value)
        }
        _ => panic!(),
    };

    // In the case of an already-empty list, value is None, so we don't need to
    // remove any inclusions.
    value.map(|value| {
        remove_inclusion(slot_map, value, list, Route::List(ListRoute { index }));
    });
}

fn set_insert(slot_map: &mut SlotMap<ArenaKey, Value>, set: ArenaKey, value: ArenaKey) {
    add_inclusion(slot_map, value, set, Route::Set);
    match &mut slot_map.get_mut(set).unwrap().structure {
        Structure::Set(hash_set) => {
            hash_set.insert(value);
        }
        _ => panic!(),
    }
}

fn set_remove(slot_map: &mut SlotMap<ArenaKey, Value>, set: ArenaKey, value: ArenaKey) {
    remove_inclusion(slot_map, value, set, Route::Set);
    match &mut slot_map.get_mut(set).unwrap().structure {
        Structure::Set(hash_set) => {
            hash_set.remove(&value);
        }
        _ => panic!(),
    }
}

fn set_union(slot_map: &mut SlotMap<ArenaKey, Value>, set_to_modify: ArenaKey, other: ArenaKey) {
    // add `set_to_modify` to the inclusions of the indications of `other`
    let other_indications = match &slot_map.get(other).unwrap().structure {
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
        }] => match [set_to_modify, other] {
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
    let other_indications = match &slot_map.get(other).unwrap().structure {
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
        }] => match [set_to_modify, other] {
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
    match &slot_map.get(map).unwrap().structure {
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
    match &mut slot_map.get_mut(map).unwrap().structure {
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
    match &mut slot_map.get_mut(map).unwrap().structure {
        Structure::Map(hash_map) => {
            hash_map.insert(key, value);
        }
        _ => panic!(),
    }
}

fn map_get(slot_map: &SlotMap<ArenaKey, Value>, map: ArenaKey, key: ArenaKey) -> Option<ArenaKey> {
    match &slot_map.get(map).unwrap().structure {
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
            lookup_map,
        }
    }

    pub fn string(&mut self, string: &str) -> ArenaKey {
        insert_string(&mut self.slot_map, &mut self.lookup_map, string)
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

    pub fn list_push<S: Into<String>>(
        &mut self,
        list_register: S,
        value_register: S,
    ) -> Option<()> {
        let list_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            list_register.into(),
        );
        let value_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            value_register.into(),
        );

        let list = map_get(&self.slot_map, self.register_map, list_register)?;
        let value = map_get(&self.slot_map, self.register_map, value_register)?;

        list_push(&mut self.slot_map, list, value);

        Some(())
    }

    pub fn list_pop<S: Into<String>>(
        &mut self,
        list_register: S,
    ) -> Option<()> {
        let list_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            list_register.into(),
        );

        let list = map_get(&self.slot_map, self.register_map, list_register)?;

        list_pop(&mut self.slot_map, list);

        Some(())
    }

    pub fn set_insert_string<S: Into<String>>(&mut self, set_register: S, string: S) -> Option<()> {
        let set_register = insert_string(
            &mut self.slot_map,
            &mut self.lookup_map,
            set_register.into(),
        );
        let string = insert_string(&mut self.slot_map, &mut self.lookup_map, string.into());

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
