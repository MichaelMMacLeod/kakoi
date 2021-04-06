use std::collections::VecDeque;

use crate::{
    circle::{Circle, CirclePositioner, Point},
    render::{
        circle::{CircleConstraintBuilder, MIN_RADIUS},
        // image::ImageRenderer,
        text::TextConstraintBuilder,
    },
    sphere::Sphere,
};

macro_rules! implement_key_types {
    ( $( { $x:ident $accessor:ident } )* ) => {
        $(
            #[derive(PartialEq, Eq, Debug, Clone, Copy)]
            pub struct $x {
                index: usize,
            }

            impl From<usize> for $x {
                fn from(index: usize) -> Self {
                    Self { index }
                }
            }

            impl From<$x> for Key {
                fn from(x: $x) -> Key {
                    Key::$x(x)
                }
            }
        )*

        #[derive(PartialEq, Eq, Debug, Clone, Copy)]
        pub enum Key {
            $(
                $x($x),
            )*
        }

        impl Key {
            pub fn index(&self) -> usize {
                match self {
                    $(
                        Self::$x(x) => x.index,
                    )*
                }
            }

            $(
                #[allow(unused)]
                fn $accessor(&self) -> Option<&$x> {
                    match self {
                        Self::$x(x) => Some(x),
                        _ => None,
                    }
                }
            )*
        }
    }
}

implement_key_types! {
    {SetKey set_key}
    {IndicationTreeKey indication_tree_key}
    {StringKey string_key}
    {ImageKey image_key}
    {OverlayKey overlay_key}
}

#[derive(Debug)]
pub struct Set {
    indications: Vec<Key>,
    focused_indication: usize,
    zoom: f32,
}

impl IntoIterator for Set {
    type Item = Key;
    type IntoIter = std::vec::IntoIter<Key>;
    fn into_iter(self) -> Self::IntoIter {
        self.indications.into_iter()
    }
}

impl Set {
    fn new_empty() -> Self {
        Set {
            indications: vec![],
            focused_indication: 0,
            zoom: 0.0,
        }
    }
    fn indicate(&mut self, key: Key) -> usize {
        let v = self.indications.len();
        self.indications.push(key);
        v
    }
    fn forget(&mut self, index: usize) {
        self.indications.swap_remove(index);
    }
}

#[derive(Debug)]
pub struct IndicationTree {
    pub sphere: Sphere,
    pub key: Key,
    pub indications: Vec<IndicationTreeKey>,
}

#[derive(Debug)]
pub struct Overlay {
    focus: (Key, usize),
    message: (Key, usize),
    message_visible: bool,
}

impl Overlay {
    pub fn focus(&self) -> &Key {
        &self.focus.0
    }
}

type Image = image::RgbaImage;

#[derive(Debug)]
pub enum Structure {
    Set(Set),
    IndicationTree(IndicationTree),
    Overlay(Overlay),
    String(String),
    Image(Image),
}

macro_rules! implement_structure_accessors {
    ( $( { $name:ident $mut_name:ident $t:ident } )* ) => {
        $(
            #[allow(unused)]
            fn $name(&self) -> &$t {
                match self {
                    Self::$t(x) => x,
                    _ => unreachable!(),
                }
            }
            #[allow(unused)]
            fn $mut_name(&mut self) -> &mut $t {
                match self {
                    Self::$t(x) => x,
                    _ => unreachable!(),
                }
            }
        )*
    }
}

impl Structure {
    implement_structure_accessors! {
        { unchecked_set unchecked_set_mut Set }
        { unchecked_indication_tree unchecked_indication_tree_mut IndicationTree }
        { unchecked_overlay unchecked_overlay_mut Overlay }
        { unchecked_string unchecked_string_mut String }
        { unchecked_image unchecked_image_mut Image }
    }
}

#[derive(Debug)]
pub struct Value {
    indications: Box<Structure>,
    inclusions: Set,
}

#[derive(Debug)]
pub struct Store {
    values: Vec<Option<Value>>,
    free_values: Vec<usize>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            values: vec![],
            free_values: vec![],
        }
    }

    pub fn naming_example() -> (Self, OverlayKey) {
        let mut store = Self::new();

        let consonant_set = {
            let consonants = [
                "b", "c", "d", "f", "g", "h", "j", "k", "l", "m", "n", "p", "q", "r", "s", "t",
                "v", "w", "x", "y", "z",
            ]
            .iter()
            .map(|s| Key::from(store.insert_string(s)))
            .collect::<Vec<_>>();
            store.insert_set(consonants)
        };

        let vowel_set = {
            let vowels = ["a", "e", "i", "o", "u"]
                .iter()
                .map(|s| Key::from(store.insert_string(s)))
                .collect::<Vec<_>>();
            store.insert_set(vowels)
        };

        let named_consonant_set = {
            let consonant = store.insert_string("Consonant");
            store.insert_set(vec![Key::from(consonant_set), Key::from(consonant)])
        };

        let named_vowel_set = {
            let vowel = store.insert_string("Vowel");
            store.insert_set(vec![Key::from(vowel_set), Key::from(vowel)])
        };

        let name_set = store.insert_set(vec![
            Key::from(named_consonant_set),
            Key::from(named_vowel_set),
        ]);

        let named_name_set = {
            let name = store.insert_string("Name");
            store.insert_set(vec![Key::from(name_set), Key::from(name)])
        };

        store.set_indicate(&name_set, &Key::from(named_name_set));

        let message = store.insert_string("Welcome to Kakoi");

        let overlay = store.insert_overlay(Key::from(name_set), Key::from(message), true);

        (store, overlay)
    }

    pub fn get(&self, key: Key) -> &Value {
        self.values[key.index()].as_ref().unwrap()
    }

    pub fn get_string(&self, key: &StringKey) -> &String {
        self.values[key.index]
            .as_ref()
            .unwrap()
            .indications
            .unchecked_string()
    }

    pub fn get_set(&self, key: &SetKey) -> &Set {
        self.values[key.index]
            .as_ref()
            .unwrap()
            .indications
            .unchecked_set()
    }

    pub fn get_image(&self, key: &ImageKey) -> &image::RgbaImage {
        self.values[key.index]
            .as_ref()
            .unwrap()
            .indications
            .unchecked_image()
    }

    pub fn get_overlay(&self, key: &OverlayKey) -> &Overlay {
        self.values[key.index]
            .as_ref()
            .unwrap()
            .indications
            .unchecked_overlay()
    }

    pub fn get_indication_tree(&self, key: &IndicationTreeKey) -> &IndicationTree {
        self.values[key.index]
            .as_ref()
            .unwrap()
            .indications
            .unchecked_indication_tree()
    }

    pub fn insert_string(&mut self, string: &str) -> StringKey {
        let (key, storage_instruction) = self.next_key::<StringKey>();
        let value = Value {
            indications: Box::new(Structure::String(string.into())),
            inclusions: Set::new_empty(),
        };
        self.add_value(value, key.index, storage_instruction);
        key
    }

    pub fn insert_image(&mut self, image: image::RgbaImage) -> ImageKey {
        let (key, storage_instruction) = self.next_key::<ImageKey>();
        let value = Value {
            indications: Box::new(Structure::Image(image)),
            inclusions: Set::new_empty(),
        };
        self.add_value(value, key.index, storage_instruction);
        key
    }

    pub fn insert_set(&mut self, indications: impl IntoIterator<Item = Key>) -> SetKey {
        let (key, storage_instruction) = self.next_key::<SetKey>();
        let indications = indications
            .into_iter()
            .map(|indication| {
                self.values[indication.index()]
                    .as_mut()
                    .unwrap()
                    .inclusions
                    .indicate(Key::from(key));
                indication
            })
            .collect();
        let value = Value {
            indications: Box::new(Structure::Set(Set {
                indications,
                focused_indication: 0,
                zoom: 0.0,
            })),
            inclusions: Set::new_empty(),
        };
        self.add_value(value, key.index, storage_instruction);
        key
    }

    pub fn insert_overlay(
        &mut self,
        focus: Key,
        message: Key,
        message_visible: bool,
    ) -> OverlayKey {
        let (key, storage_instruction) = self.next_key::<OverlayKey>();
        let focus_index = self
            .get_mut(Key::from(focus))
            .inclusions
            .indicate(Key::from(key));
        let message_index = self
            .get_mut(Key::from(message))
            .inclusions
            .indicate(Key::from(key));
        let value = Value {
            indications: Box::new(Structure::Overlay(Overlay {
                focus: (focus, focus_index),
                message: (message, message_index),
                message_visible,
            })),
            inclusions: Set::new_empty(),
        };
        self.add_value(value, key.index, storage_instruction);
        key
    }

    pub fn insert_indication_tree(&mut self, key: Key, sphere: Sphere) -> IndicationTreeKey {
        let (result_key, storage_instruction) = self.next_key::<IndicationTreeKey>();
        let value = Value {
            indications: Box::new(Structure::IndicationTree(IndicationTree {
                sphere,
                key,
                indications: vec![],
            })),
            inclusions: Set::new_empty(),
        };
        self.add_value(value, result_key.index, storage_instruction);
        result_key
    }

    pub fn build_indication_tree(
        &mut self,
        start_key: Key,
        device: &wgpu::Device,
        queue: &mut wgpu::Queue,
        screen_width: f32,
        screen_height: f32,
        circle_builder: &mut CircleConstraintBuilder,
        text_builder: &mut TextConstraintBuilder,
        // image_builder: &mut ImageRenderer,
    ) -> IndicationTreeKey {
        let result_key = {
            let s = Sphere {
                center: (0.0, 0.0, 0.0).into(),
                radius: 1.0,
            };
            circle_builder.with_instance(s);
            self.insert_indication_tree(
                start_key,
            s,
            )
        };

        let mut todo = VecDeque::new();
        todo.push_back(result_key);

        while let Some(indication_tree_key) = todo.pop_front() {
            let IndicationTree {
                sphere: tree_sphere,
                key: data_key,
                ..
            } = self.get_indication_tree(&indication_tree_key);

            let other_todos = match &*self.get(*data_key).indications {
                Structure::Set(s) => {
                    let Set {
                        indications,
                        focused_indication,
                        zoom,
                    } = s;
                    let focus_angle = 2.0 * std::f32::consts::PI / indications.len() as f32
                        * *focused_indication as f32;
                    let circle_positioner = CirclePositioner::new(
                        (tree_sphere.radius * MIN_RADIUS) as f64,
                        indications.len() as u64,
                        *zoom as f64,
                        Point {
                            x: tree_sphere.center.x as f64,
                            y: tree_sphere.center.y as f64,
                        },
                        focus_angle as f64,
                    );
                    let (before_focused, focused_and_after): (Vec<_>, Vec<_>) = (0..)
                        .into_iter()
                        .zip(indications.iter())
                        .partition(|(i, _)| i < focused_indication);
                    circle_positioner
                        .into_iter()
                        .zip(
                            focused_and_after
                                .into_iter()
                                .chain(before_focused.into_iter()),
                        )
                        .filter_map(|(circle, (_, node))| {
                            let Circle { center, radius } = circle;
                            let Point { x, y } = center;
                            let radius = radius as f32;

                            let other_sphere = Sphere {
                                center: cgmath::vec3(x as f32, y as f32, 0.0),
                                radius,
                            };

                            if other_sphere.screen_radius(screen_width, screen_height) > 1.0 {
                                circle_builder.with_instance(other_sphere);
                                Some((*node, other_sphere))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                }
                Structure::IndicationTree(_) => unimplemented!(),
                Structure::Overlay(o) => {
                    let Overlay {
                        focus: (focus, _),
                        message: (message, _),
                        message_visible,
                    } = o;

                    let indications = if *message_visible {
                        vec![focus, message]
                    } else {
                        vec![focus]
                    };

                    CirclePositioner::new(
                        (tree_sphere.radius * MIN_RADIUS) as f64,
                        indications.len() as u64,
                        0.5,
                        Point {
                            x: tree_sphere.center.x as f64,
                            y: tree_sphere.center.y as f64,
                        },
                        0.0,
                    )
                    .into_iter()
                    .zip(indications.into_iter())
                    .map(
                        |(
                            Circle {
                                center: Point { x, y },
                                radius,
                            },
                            indication,
                        )| {
                            let other_sphere = Sphere {
                                center: (x as f32, y as f32, 0.0).into(),
                                radius: radius as f32,
                            };
                            circle_builder.with_instance(other_sphere);
                            (*indication, other_sphere)
                        },
                    )
                    .collect::<Vec<_>>()
                }
                Structure::String(_) => {
                    text_builder.with_instance(*tree_sphere, *data_key.string_key().unwrap());
                    vec![]
                }
                Structure::Image(_) => {
                    // image_builder.with_image(*tree_sphere, *data_key.image_key().unwrap());
                    vec![]
                }
            };

            other_todos.into_iter().for_each(|(key, sphere)| {
                let sub_tree_key = self.insert_indication_tree(key, sphere);
                self.get_indication_tree_mut(&indication_tree_key)
                    .indications
                    .push(sub_tree_key);
                todo.push_back(sub_tree_key);
            });
        }

        result_key
    }

    pub fn set_indicate(&mut self, set_key: &SetKey, key: &Key) {
        let set = self.get_set_mut(set_key);
        set.indicate(*key);
        self.get_mut(*key).inclusions.indicate(Key::from(*set_key));
    }

    pub fn overlay_indicate_focus(&mut self, overlay_key: &OverlayKey, new_focus_key: &Key) {
        let (focus_key, focus_index) = {
            let overlay = self.get_overlay(overlay_key);
            overlay.focus
        };
        self.get_mut(Key::from(focus_key))
            .inclusions
            .forget(focus_index);
        let new_focus_index = self.get_mut(*new_focus_key).inclusions.indicate(focus_key);
        let overlay = self.get_overlay_mut(overlay_key);
        overlay.focus.0 = *new_focus_key;
        overlay.focus.1 = new_focus_index;
    }

    pub fn remove_indication_tree(&mut self, indication_tree_key: IndicationTreeKey) {
        let mut todo = VecDeque::new();
        todo.push_back(indication_tree_key);

        while let Some(indication_tree_key) = todo.pop_front() {
            let indications = &self.get_indication_tree(&indication_tree_key).indications;
            for indication_tree_key in indications {
                todo.push_back(*indication_tree_key)
            }
            self.values[indication_tree_key.index] = None;
            self.free_values.push(indication_tree_key.index);
        }
    }

    fn next_key<T: From<usize>>(&mut self) -> (T, StorageInstruction) {
        match self.free_values.pop() {
            Some(index) => (T::from(index), StorageInstruction::GetMut),
            None => (T::from(self.values.len()), StorageInstruction::Push),
        }
    }

    fn add_value(
        &mut self,
        value: Value,
        value_index: usize,
        storage_instruction: StorageInstruction,
    ) {
        match storage_instruction {
            StorageInstruction::GetMut => {
                let slot = self.values.get_mut(value_index).unwrap();
                match slot {
                    Some(_) => panic!("free slot was still occupied"),
                    None => {
                        *slot = Some(value);
                    }
                }
            }
            StorageInstruction::Push => {
                self.values.push(Some(value));
            }
        }
    }

    fn get_mut(&mut self, key: Key) -> &mut Value {
        self.values[key.index()].as_mut().unwrap()
    }

    #[allow(unused)]
    fn get_string_mut(&mut self, key: &StringKey) -> &mut String {
        self.values[key.index]
            .as_mut()
            .unwrap()
            .indications
            .unchecked_string_mut()
    }

    fn get_set_mut(&mut self, key: &SetKey) -> &mut Set {
        self.values[key.index]
            .as_mut()
            .unwrap()
            .indications
            .unchecked_set_mut()
    }

    #[allow(unused)]
    fn get_image_mut(&mut self, key: &ImageKey) -> &mut image::RgbaImage {
        self.values[key.index]
            .as_mut()
            .unwrap()
            .indications
            .unchecked_image_mut()
    }

    fn get_overlay_mut(&mut self, key: &OverlayKey) -> &mut Overlay {
        self.values[key.index]
            .as_mut()
            .unwrap()
            .indications
            .unchecked_overlay_mut()
    }

    fn get_indication_tree_mut(&mut self, key: &IndicationTreeKey) -> &mut IndicationTree {
        self.values[key.index]
            .as_mut()
            .unwrap()
            .indications
            .unchecked_indication_tree_mut()
    }
}

enum StorageInstruction {
    Push,
    GetMut,
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_0() {
        eprintln!("{}", std::mem::size_of::<Value>());
        // for (value, i) in Store::naming_example().values.iter().zip(0..) {
        //     dbg!(i, value);
        // }
    }
}

// pub enum Group {
//     Existing { key: SetKey, index: usize },
//     New,
// }

// pub enum Insertion {
//     Existing(Key),
//     New(Structure),
// }

// pub fn enclose(&mut self, into: Group, insertions: Vec<Insertion>) -> Option<Key> {
//     match into {
//         Group::New => match insertions.len() {
//             0 => None,
//             1 => match insertions[0] {
//                 Insertion::Existing(key) => Some(key),
//                 Insertion::New(s) => {
//                     let key = self.next_key_from_structure(&s);
//                     let value = Value {
//                         indications: s,
//                         inclusions: Set::new_empty(),
//                     };
//                     self.values.push(value);
//                     Some(key)
//                 }
//             },
//             _ => {

//             }
//             // 1 => Some(self.get_target(store, insertions.drain(..).next().unwrap())),
//             // _ => Some(self.create_association(store, insertions)),
//         },
//         Group::Existing { key: source, index } => match insertions.len() {
//             0 => Some(source),
//             1 => {
//                 let target = self.get_target(store, insertions.drain(..).next().unwrap());
//                 self.insert_target_at(store, source, target, index);
//                 Some(source)
//             }
//             _ => {
//                 let target = self.create_association(store, insertions);
//                 self.insert_target_at(store, source, target, index);
//                 Some(source)
//             }
//         },
//     }
// }

// fn next_key_from_structure(&self, structure: &Structure) -> Key {
//     match structure {
//         Structure::Set(_) => Key::SetKey(SetKey::from(self.values.len())),
//         Structure::String(_) => Key::StringKey(StringKey::from(self.values.len())),
//         Structure::Image(_) => Key::ImageKey(ImageKey::from(self.values.len())),
//     }
// }
