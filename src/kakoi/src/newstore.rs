macro_rules! implement_key_types {
    ( $($x:ident)* ) => {
        $(
            #[derive(Debug, Clone, Copy)]
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

        #[derive(Debug, Clone, Copy)]
        pub enum Key {
            $(
                $x($x),
            )*
        }

        impl Key {
            fn index(&self) -> usize {
                match self {
                    $(
                        Self::$x(x) => x.index,
                    )*
                }
            }
        }
    }
}

implement_key_types! { SetKey StringKey ImageKey OverlayKey }

#[derive(Debug)]
pub struct Set {
    indications: Vec<Key>,
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
pub struct Overlay {
    focus: (Key, usize),
    message: (Key, usize),
    message_visible: bool,
}

#[derive(Debug)]
pub enum Structure {
    Set(Set),
    Overlay(Overlay),
    String(String),
    Image(image::RgbaImage),
}

impl Structure {
    fn unchecked_set(&self) -> &Set {
        match self {
            Self::Set(s) => s,
            _ => unreachable!(),
        }
    }

    fn unchecked_string(&self) -> &String {
        match self {
            Self::String(s) => s,
            _ => unreachable!(),
        }
    }

    fn unchecked_image(&self) -> &image::RgbaImage {
        match self {
            Self::Image(s) => s,
            _ => unreachable!(),
        }
    }

    fn unchecked_overlay(&self) -> &Overlay {
        match self {
            Self::Overlay(s) => s,
            _ => unreachable!(),
        }
    }

    fn unchecked_set_mut(&mut self) -> &mut Set {
        match self {
            Self::Set(s) => s,
            _ => unreachable!(),
        }
    }

    #[allow(unused)]
    fn unchecked_string_mut(&mut self) -> &mut String {
        match self {
            Self::String(s) => s,
            _ => unreachable!(),
        }
    }

    #[allow(unused)]
    fn unchecked_image_mut(&mut self) -> &mut image::RgbaImage {
        match self {
            Self::Image(s) => s,
            _ => unreachable!(),
        }
    }

    fn unchecked_overlay_mut(&mut self) -> &mut Overlay {
        match self {
            Self::Overlay(s) => s,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct Value {
    indications: Structure,
    inclusions: Set,
}

#[derive(Debug)]
pub struct Store {
    values: Vec<Value>,
}

impl Store {
    pub fn new() -> Self {
        Self { values: Vec::new() }
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
        &self.values[key.index()]
    }

    pub fn get_string(&self, key: &StringKey) -> &String {
        self.values[key.index].indications.unchecked_string()
    }

    pub fn get_set(&self, key: &SetKey) -> &Set {
        self.values[key.index].indications.unchecked_set()
    }

    pub fn get_image(&self, key: &ImageKey) -> &image::RgbaImage {
        self.values[key.index].indications.unchecked_image()
    }

    pub fn get_overlay(&self, key: &OverlayKey) -> &Overlay {
        self.values[key.index].indications.unchecked_overlay()
    }

    pub fn insert_string(&mut self, string: &str) -> StringKey {
        let key = self.next_key();
        let value = Value {
            indications: Structure::String(string.into()),
            inclusions: Set::new_empty(),
        };
        self.values.push(value);
        key
    }

    pub fn insert_image(&mut self, image: image::RgbaImage) -> ImageKey {
        let key = self.next_key();
        let value = Value {
            indications: Structure::Image(image),
            inclusions: Set::new_empty(),
        };
        self.values.push(value);
        key
    }

    pub fn insert_set(&mut self, indications: impl IntoIterator<Item = Key>) -> SetKey {
        let key = self.next_key();
        let indications = indications
            .into_iter()
            .map(|indication| {
                self.values[indication.index()]
                    .inclusions
                    .indicate(Key::from(key));
                indication
            })
            .collect();
        let value = Value {
            indications: Structure::Set(Set { indications }),
            inclusions: Set::new_empty(),
        };
        self.values.push(value);
        key
    }

    pub fn insert_overlay(
        &mut self,
        focus: Key,
        message: Key,
        message_visible: bool,
    ) -> OverlayKey {
        let key = self.next_key();
        let focus_index = self
            .get_mut(Key::from(focus))
            .inclusions
            .indicate(Key::from(key));
        let message_index = self
            .get_mut(Key::from(message))
            .inclusions
            .indicate(Key::from(key));
        let value = Value {
            indications: Structure::Overlay(Overlay {
                focus: (focus, focus_index),
                message: (message, message_index),
                message_visible,
            }),
            inclusions: Set::new_empty(),
        };
        self.values.push(value);
        key
    }

    pub fn set_indicate(&mut self, set_key: &SetKey, key: &Key) {
        let set = self.get_set_mut(set_key);
        set.indicate(*key);
        self.get_mut(*key).inclusions.indicate(Key::from(*set_key));
    }

    pub fn overlay_indicate_focus(
        &mut self,
        overlay_key: &OverlayKey,
        new_focus_key: &Key,
        new_focus_index: usize,
    ) {
        let (focus_key, focus_index) = {
            let overlay = self.get_overlay(overlay_key);
            overlay.focus
        };
        self.get_mut(Key::from(focus_key))
            .inclusions
            .forget(focus_index);
        let overlay = self.get_overlay_mut(overlay_key);
        overlay.focus.0 = *new_focus_key;
        overlay.focus.1 = new_focus_index;
    }

    fn next_key<T: From<usize>>(&self) -> T {
        T::from(self.values.len())
    }

    fn get_mut(&mut self, key: Key) -> &mut Value {
        &mut self.values[key.index()]
    }

    #[allow(unused)]
    fn get_string_mut(&mut self, key: &StringKey) -> &mut String {
        self.values[key.index].indications.unchecked_string_mut()
    }

    fn get_set_mut(&mut self, key: &SetKey) -> &mut Set {
        self.values[key.index].indications.unchecked_set_mut()
    }

    #[allow(unused)]
    fn get_image_mut(&mut self, key: &ImageKey) -> &mut image::RgbaImage {
        self.values[key.index].indications.unchecked_image_mut()
    }

    fn get_overlay_mut(&mut self, key: &OverlayKey) -> &mut Overlay {
        self.values[key.index].indications.unchecked_overlay_mut()
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     #[test]
//     fn test_0() {
//         for (value, i) in Store::naming_example().values.iter().zip(0..) {
//             dbg!(i, value);
//         }
//     }
// }

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
