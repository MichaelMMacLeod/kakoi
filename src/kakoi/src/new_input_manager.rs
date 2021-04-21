use crate::input_manager::CompleteAction;
use slotmap::{new_key_type, SlotMap};
use std::collections::{HashMap, VecDeque};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

pub struct InputManager {
    key_binder: KeyBinder,
    input_state: InputState,
    pressed_keys: PressedKeys,
}

impl InputManager {
    pub fn new() -> Self {
        let mut key_binder = KeyBinder::new();
        key_binder.with_default_bindings();
        let input_state = key_binder.start_state().unwrap();
        Self {
            key_binder,
            input_state,
            pressed_keys: PressedKeys {
                shift_pressed: false,
            },
        }
    }
    pub fn process_input(&mut self, keyboard_input: &KeyboardInput) -> Option<CompleteAction> {
        let pressed = keyboard_input.state == ElementState::Pressed;
        if let Some(virtual_key_code) = &keyboard_input.virtual_keycode {
            match virtual_key_code {
                VirtualKeyCode::LShift | VirtualKeyCode::RShift => {
                    self.pressed_keys.shift_pressed = pressed
                }
                _ => {}
            }

            let input = Input {
                virtual_key_code,
                pressed_keys: &self.pressed_keys,
            };

            if pressed {
                self.key_binder.process_input(&mut self.input_state, input)
            } else {
                None
            }
        } else {
            None
        }
    }
}

enum InputAccumulationStage {
    InputRequirement(InputRequirement),
    Done(fn(&mut Vec<String>) -> CompleteAction),
}

pub struct KeyBinder {
    slot_map: SlotMap<InputManagerKey, InputAccumulationStage>,
    start_stage: Option<InputManagerKey>,
}

impl KeyBinder {
    fn new() -> Self {
        Self {
            slot_map: SlotMap::with_key(),
            start_stage: None,
        }
    }

    fn with_default_bindings(&mut self) {
        self.bind(vec![key("s"), register()], |v| {
            let register = v.pop().unwrap();
            CompleteAction::SelectRegister(register)
        });
        self.bind(vec![key("e")], |_| {
            CompleteAction::BindRegisterToEmptySet(".".into())
        });
        self.bind(vec![key("v")], |_| CompleteAction::Registers);
        self.bind(vec![key("p")], |_| CompleteAction::Back);
        self.bind(vec![key("t"), string()], |v| {
            let string = v.pop().unwrap();
            CompleteAction::InsertStringIntoSetRegister(".".into(), string)
        });
        self.bind(vec![key("i"), register()], |v| {
            let register = v.pop().unwrap();
            CompleteAction::SetInsert(".".into(), register)
        });
        self.bind(vec![key("b"), register()], |v| {
            let register_to_bind = v.pop().unwrap();
            CompleteAction::BindRegisterToRegisterValue(register_to_bind, ".".into())
        });
        self.bind(vec![key("r"), register()], |v| {
            let register = v.pop().unwrap();
            CompleteAction::SetRemove(".".into(), register)
        });
    }

    fn bind(
        &mut self,
        route: Vec<InputRequirementDescriptor>,
        action_constructor: fn(&mut Vec<String>) -> CompleteAction,
    ) {
        let action_key = self
            .slot_map
            .insert(InputAccumulationStage::Done(action_constructor));
        let first_key =
            route
                .into_iter()
                .rev()
                .fold(action_key, |next_key, processing_descriptor| {
                    self.slot_map
                        .insert(InputAccumulationStage::InputRequirement(
                            processing_descriptor.to_input_requirement(next_key),
                        ))
                });
        match self.start_stage {
            Some(start_state) => {
                recursively_merge(
                    &mut self.slot_map,
                    &mut vec![Merge {
                        into: start_state,
                        from: first_key,
                    }]
                    .into_iter()
                    .collect(),
                );
            }
            None => self.start_stage = Some(first_key),
        }
    }

    fn start_state(&self) -> Option<InputState> {
        self.start_stage.map(|start_state| InputState {
            current_stage: start_state,
            processed_input: vec![],
            current_processor: None,
        })
    }

    fn process_input(&self, input_state: &mut InputState, input: Input) -> Option<CompleteAction> {
        match self.slot_map.get(input_state.current_stage).unwrap() {
            InputAccumulationStage::InputRequirement(processing) => {
                let processor = input_state
                    .current_processor
                    .get_or_insert_with(|| processing.recorder());
                let mut next_value = None;
                let mut next_stage = None;
                processor.process(input).map(|result| {
                    next_stage = Some(processing.next_state(&result)).unwrap();
                    next_value = Some(result);
                });
                next_value.map(|next_value| {
                    input_state.processed_input.push(next_value);
                    input_state.current_processor = None;
                });
                next_stage.map(|next_stage| input_state.current_stage = *next_stage);
                if let InputAccumulationStage::Done(action_constructor) =
                    self.slot_map.get(input_state.current_stage).unwrap()
                {
                    let complete_action = action_constructor(&mut input_state.processed_input);
                    input_state.processed_input.clear();
                    input_state.current_stage = self.start_stage.unwrap();
                    input_state.current_processor = None;
                    Some(complete_action)
                } else {
                    None
                }
            }
            InputAccumulationStage::Done(_) => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InputState {
    current_stage: InputManagerKey,
    current_processor: Option<InputProcessor>,
    processed_input: Vec<String>,
}

new_key_type! {
    pub struct InputManagerKey;
}

pub fn key(str: &str) -> InputRequirementDescriptor {
    InputRequirementDescriptor::Key(str.to_owned())
}

pub fn string() -> InputRequirementDescriptor {
    InputRequirementDescriptor::String
}

pub fn register() -> InputRequirementDescriptor {
    InputRequirementDescriptor::Register
}

pub enum InputRequirementDescriptor {
    Register,
    String,
    Key(String),
}

impl InputRequirementDescriptor {
    fn to_input_requirement(self, key: InputManagerKey) -> InputRequirement {
        match self {
            Self::Register => InputRequirement::Register(key),
            Self::String => InputRequirement::String(key),
            Self::Key(code) => InputRequirement::Key(vec![(code, key)].into_iter().collect()),
        }
    }
}

enum InputRequirement {
    Register(InputManagerKey),
    String(InputManagerKey),
    Key(HashMap<String, InputManagerKey>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct StringProcessor {
    string: String,
    done: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum InputProcessor {
    Register,
    Key,
    String(StringProcessor),
}

struct PressedKeys {
    shift_pressed: bool,
}

pub struct Input<'a> {
    pressed_keys: &'a PressedKeys,
    virtual_key_code: &'a VirtualKeyCode,
}

impl InputProcessor {
    fn process(&mut self, input: Input) -> Option<String> {
        match self {
            Self::Register => {
                Some(crate::input_map::vk_to_keyname_string(input.virtual_key_code).into())
            }
            Self::Key => {
                Some(crate::input_map::vk_to_keyname_string(input.virtual_key_code).into())
            }
            Self::String(StringProcessor { string, done }) => {
                enum Do {
                    Insert(String),
                    Delete(bool),
                    Done,
                    Nothing,
                }

                let fix_case = |str: &'static str| -> String {
                    if input.pressed_keys.shift_pressed {
                        str.to_uppercase()
                    } else {
                        str.into()
                    }
                };

                let enter = || -> Do {
                    if input.pressed_keys.shift_pressed {
                        Do::Done
                    } else {
                        Do::Insert("\n".into())
                    }
                };

                let d = match input.virtual_key_code {
                    VirtualKeyCode::Key1 => Do::Insert("1".into()),
                    VirtualKeyCode::Key2 => Do::Insert("2".into()),
                    VirtualKeyCode::Key3 => Do::Insert("3".into()),
                    VirtualKeyCode::Key4 => Do::Insert("4".into()),
                    VirtualKeyCode::Key5 => Do::Insert("5".into()),
                    VirtualKeyCode::Key6 => Do::Insert("6".into()),
                    VirtualKeyCode::Key7 => Do::Insert("7".into()),
                    VirtualKeyCode::Key8 => Do::Insert("8".into()),
                    VirtualKeyCode::Key9 => Do::Insert("9".into()),
                    VirtualKeyCode::Key0 => Do::Insert("0".into()),
                    VirtualKeyCode::A => Do::Insert(fix_case("a")),
                    VirtualKeyCode::B => Do::Insert(fix_case("b")),
                    VirtualKeyCode::C => Do::Insert(fix_case("c")),
                    VirtualKeyCode::D => Do::Insert(fix_case("d")),
                    VirtualKeyCode::E => Do::Insert(fix_case("e")),
                    VirtualKeyCode::F => Do::Insert(fix_case("f")),
                    VirtualKeyCode::G => Do::Insert(fix_case("g")),
                    VirtualKeyCode::H => Do::Insert(fix_case("h")),
                    VirtualKeyCode::I => Do::Insert(fix_case("i")),
                    VirtualKeyCode::J => Do::Insert(fix_case("j")),
                    VirtualKeyCode::K => Do::Insert(fix_case("k")),
                    VirtualKeyCode::L => Do::Insert(fix_case("l")),
                    VirtualKeyCode::M => Do::Insert(fix_case("m")),
                    VirtualKeyCode::N => Do::Insert(fix_case("n")),
                    VirtualKeyCode::O => Do::Insert(fix_case("o")),
                    VirtualKeyCode::P => Do::Insert(fix_case("p")),
                    VirtualKeyCode::Q => Do::Insert(fix_case("q")),
                    VirtualKeyCode::R => Do::Insert(fix_case("r")),
                    VirtualKeyCode::S => Do::Insert(fix_case("s")),
                    VirtualKeyCode::T => Do::Insert(fix_case("t")),
                    VirtualKeyCode::U => Do::Insert(fix_case("u")),
                    VirtualKeyCode::V => Do::Insert(fix_case("v")),
                    VirtualKeyCode::W => Do::Insert(fix_case("w")),
                    VirtualKeyCode::X => Do::Insert(fix_case("x")),
                    VirtualKeyCode::Y => Do::Insert(fix_case("y")),
                    VirtualKeyCode::Z => Do::Insert(fix_case("z")),
                    VirtualKeyCode::Delete => Do::Delete(input.pressed_keys.shift_pressed),
                    VirtualKeyCode::Return => enter(),
                    VirtualKeyCode::Space => Do::Insert(" ".into()),
                    VirtualKeyCode::Numpad0 => Do::Insert("0".into()),
                    VirtualKeyCode::Numpad1 => Do::Insert("1".into()),
                    VirtualKeyCode::Numpad2 => Do::Insert("2".into()),
                    VirtualKeyCode::Numpad3 => Do::Insert("3".into()),
                    VirtualKeyCode::Numpad4 => Do::Insert("4".into()),
                    VirtualKeyCode::Numpad5 => Do::Insert("5".into()),
                    VirtualKeyCode::Numpad6 => Do::Insert("6".into()),
                    VirtualKeyCode::Numpad7 => Do::Insert("7".into()),
                    VirtualKeyCode::Numpad8 => Do::Insert("8".into()),
                    VirtualKeyCode::Numpad9 => Do::Insert("9".into()),
                    VirtualKeyCode::Divide => Do::Insert("/".into()),
                    VirtualKeyCode::Decimal => Do::Insert(".".into()),
                    VirtualKeyCode::NumpadComma => Do::Insert(",".into()),
                    VirtualKeyCode::NumpadEnter => enter(),
                    VirtualKeyCode::NumpadEquals => Do::Insert("=".into()),
                    VirtualKeyCode::Multiply => Do::Insert("*".into()),
                    VirtualKeyCode::Subtract => Do::Insert("-".into()),
                    VirtualKeyCode::Apostrophe => Do::Insert("'".into()),
                    VirtualKeyCode::At => Do::Insert("@".into()),
                    VirtualKeyCode::Backslash => Do::Insert("\\".into()),
                    VirtualKeyCode::Colon => Do::Insert(":".into()),
                    VirtualKeyCode::Comma => Do::Insert(",".into()),
                    VirtualKeyCode::Equals => Do::Insert("=".into()),
                    VirtualKeyCode::Grave => Do::Insert("`".into()),
                    VirtualKeyCode::LBracket => Do::Insert("[".into()),
                    VirtualKeyCode::Minus => Do::Insert("-".into()),
                    VirtualKeyCode::Period => Do::Insert(".".into()),
                    VirtualKeyCode::Add => Do::Insert("+".into()),
                    VirtualKeyCode::RBracket => Do::Insert("]".into()),
                    VirtualKeyCode::Semicolon => Do::Insert(";".into()),
                    VirtualKeyCode::Slash => Do::Insert("/".into()),
                    VirtualKeyCode::Tab => Do::Insert("\t".into()),
                    VirtualKeyCode::Underline => Do::Insert("_".into()),
                    _ => Do::Nothing,
                };
                match d {
                    Do::Insert(mut to_append) => {
                        string.push_str(&mut to_append);
                    }
                    Do::Delete(delete_entire_word) => {
                        // there's got to be a better way, right?
                        *string = if delete_entire_word {
                            unicode_segmentation::UnicodeSegmentation::split_word_bounds(
                                string.as_str(),
                            )
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .skip(1)
                            .rev()
                            .collect::<String>()
                        } else {
                            unicode_segmentation::UnicodeSegmentation::graphemes(
                                string.as_str(),
                                true,
                            )
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .skip(1)
                            .rev()
                            .collect::<String>()
                        };
                    }
                    Do::Done => *done = true,
                    Do::Nothing => {}
                }

                if *done {
                    Some(string.clone())
                } else {
                    None
                }
            }
        }
    }
}

struct Merge {
    into: InputManagerKey,
    from: InputManagerKey,
}

fn recursively_merge(
    slot_map: &mut SlotMap<InputManagerKey, InputAccumulationStage>,
    todo: &mut VecDeque<Merge>,
) {
    while let Some(Merge { into, from }) = todo.pop_front() {
        match slot_map.get_disjoint_mut([into, from]).unwrap() {
            [InputAccumulationStage::InputRequirement(InputRequirement::Key(into_map)), InputAccumulationStage::InputRequirement(InputRequirement::Key(from_map))] =>
            {
                for (from_k, from_v) in from_map.drain() {
                    match into_map.get(&from_k) {
                        Some(into_v) => {
                            todo.push_back(Merge {
                                into: *into_v,
                                from: from_v,
                            });
                        }
                        None => {
                            into_map.insert(from_k, from_v);
                        }
                    }
                }
                slot_map.remove(from);
            }
            _ => panic!(),
        }
    }
}

impl InputRequirement {
    fn next_state(&self, data: &String) -> Option<&InputManagerKey> {
        match self {
            InputRequirement::Register(k) => Some(k),
            InputRequirement::String(k) => Some(k),
            InputRequirement::Key(map) => map.get(data),
        }
    }
    fn recorder(&self) -> InputProcessor {
        match self {
            Self::Register(_) => InputProcessor::Register,
            Self::Key(_) => InputProcessor::Key,
            Self::String(_) => InputProcessor::String(StringProcessor {
                string: "".into(),
                done: false,
            }),
        }
    }
}
