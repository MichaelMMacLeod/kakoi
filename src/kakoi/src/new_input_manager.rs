use crate::input_manager::CompleteAction;
use slotmap::{new_key_type, SlotMap};
use std::collections::{HashMap, VecDeque};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

pub struct InputMagic {
    input_manager: InputManager,
    input_state: InputState,
    modifiers: Modifiers,
}

impl InputMagic {
    pub fn new() -> Self {
        let mut input_manager = InputManager::new();
        input_manager.bind_defaults();
        let input_state = input_manager.start().unwrap();
        Self {
            input_manager,
            input_state,
            modifiers: Modifiers {
                shift_pressed: false,
            },
        }
    }
    pub fn input(&mut self, keyboard_input: &KeyboardInput) -> Option<CompleteAction> {
        let pressed = keyboard_input.state == ElementState::Pressed;
        if let Some(virtual_key_code) = &keyboard_input.virtual_keycode {
            match virtual_key_code {
                VirtualKeyCode::LShift | VirtualKeyCode::RShift => {
                    self.modifiers.shift_pressed = pressed
                }
                _ => {}
            }

            let input = Input {
                virtual_key_code,
                modifiers: &self.modifiers,
            };

            if pressed {
                self.input_manager.input(&mut self.input_state, input)
            } else {
                None
            }
        } else {
            None
        }
    }
}

enum InputManagerNode {
    Processing(Processing),
    Done(fn(&mut Vec<String>) -> CompleteAction),
}

pub struct InputManager {
    slot_map: SlotMap<InputManagerKey, InputManagerNode>,
    start_state: Option<InputManagerKey>,
}

pub enum InputResult {
    InputState(InputState),
    CompleteAction(CompleteAction),
}

// keycombo! { Key("space"), Key("x"), Register, Register }
// macro_rules! keycombo {
//     ($($v:expr),+ $(,)?) => {
//         vec![$(ProcessingDescriptor::$v),+]
//     }
// }

impl InputManager {
    pub fn new() -> Self {
        Self {
            slot_map: SlotMap::with_key(),
            start_state: None,
        }
    }

    pub fn bind_defaults(&mut self) {
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
        // self.bind(
        //     vec![
        //         key("space"),
        //         key("r"),
        //         key("b"),
        //         key("t"),
        //         register(),
        //         string(),
        //     ],
        //     |mut v| {
        //         let string = v.pop().unwrap();
        //         let register = v.pop().unwrap();
        //         CompleteAction::BindRegisterToString(register, string)
        //     },
        // )
    }

    pub fn bind(
        &mut self,
        route: Vec<ProcessingDescriptor>,
        action_constructor: fn(&mut Vec<String>) -> CompleteAction,
    ) {
        let action_key = self
            .slot_map
            .insert(InputManagerNode::Done(action_constructor));
        let first_key =
            route
                .into_iter()
                .rev()
                .fold(action_key, |next_key, processing_descriptor| {
                    self.slot_map.insert(InputManagerNode::Processing(
                        processing_descriptor.build(next_key),
                    ))
                });
        match self.start_state {
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
            None => self.start_state = Some(first_key),
        }
    }

    pub fn start(&self) -> Option<InputState> {
        self.start_state.map(|start_state| InputState {
            state: start_state,
            accumulator: vec![],
            recorder: None,
        })
    }

    pub fn input(&self, input_state: &mut InputState, input: Input) -> Option<CompleteAction> {
        match self.slot_map.get(input_state.state).unwrap() {
            InputManagerNode::Processing(processing) => {
                let recorder = input_state
                    .recorder
                    .get_or_insert_with(|| processing.recorder());
                let mut next_value = None;
                let mut next_state = None;
                recorder.record(input).map(|result| {
                    next_state = Some(processing.next_state(&result)).unwrap();
                    next_value = Some(result);
                });
                next_value.map(|next_value| {
                    input_state.accumulator.push(next_value);
                    input_state.recorder = None;
                });
                next_state.map(|next_state| input_state.state = *next_state);
                if let InputManagerNode::Done(action_constructor) =
                    self.slot_map.get(input_state.state).unwrap()
                {
                    let complete = action_constructor(&mut input_state.accumulator);
                    input_state.accumulator.clear();
                    input_state.state = self.start_state.unwrap();
                    input_state.recorder = None;
                    Some(complete)
                } else {
                    None
                }
            }
            InputManagerNode::Done(_) => panic!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InputState {
    state: InputManagerKey,
    accumulator: Vec<String>,
    recorder: Option<Recorder>,
}

new_key_type! {
    pub struct InputManagerKey;
}

pub fn key(str: &str) -> ProcessingDescriptor {
    ProcessingDescriptor::Key(str.to_owned())
}

pub fn string() -> ProcessingDescriptor {
    ProcessingDescriptor::String
}

pub fn register() -> ProcessingDescriptor {
    ProcessingDescriptor::Register
}

pub enum ProcessingDescriptor {
    Register,
    String,
    Key(String),
}

impl ProcessingDescriptor {
    fn build(self, key: InputManagerKey) -> Processing {
        match self {
            Self::Register => Processing::Register(key),
            Self::String => Processing::String(key),
            Self::Key(code) => Processing::Key(vec![(code, key)].into_iter().collect()),
        }
    }
}

enum Processing {
    Register(InputManagerKey),
    String(InputManagerKey),
    Key(HashMap<String, InputManagerKey>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct StringRecorder {
    string: String,
    done: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum Recorder {
    Register(Option<String>),
    String(StringRecorder),
    Key(Option<String>),
}

struct Modifiers {
    shift_pressed: bool,
}

pub struct Input<'a> {
    modifiers: &'a Modifiers,
    virtual_key_code: &'a VirtualKeyCode,
}

impl Recorder {
    fn record(&mut self, input: Input) -> Option<String> {
        match self {
            Self::Register(s) => {
                Some(crate::input_map::vk_to_keyname_string(input.virtual_key_code).into())
            }
            Self::Key(s) => {
                Some(crate::input_map::vk_to_keyname_string(input.virtual_key_code).into())
            }
            Self::String(StringRecorder { string, done }) => {
                enum Do {
                    Insert(String),
                    Delete(bool),
                    Done,
                    Nothing,
                }

                let fix_case = |str: &'static str| -> String {
                    if input.modifiers.shift_pressed {
                        str.to_uppercase()
                    } else {
                        str.into()
                    }
                };

                let enter = || -> Do {
                    if input.modifiers.shift_pressed {
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
                    VirtualKeyCode::Delete => Do::Delete(input.modifiers.shift_pressed),
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
    slot_map: &mut SlotMap<InputManagerKey, InputManagerNode>,
    todo: &mut VecDeque<Merge>,
) {
    while let Some(Merge { into, from }) = todo.pop_front() {
        match slot_map.get_disjoint_mut([into, from]).unwrap() {
            [InputManagerNode::Processing(Processing::Key(into_map)), InputManagerNode::Processing(Processing::Key(from_map))] =>
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

impl Processing {
    fn next_state(&self, data: &String) -> Option<&InputManagerKey> {
        match self {
            Processing::Register(k) => Some(k),
            Processing::String(k) => Some(k),
            Processing::Key(map) => map.get(data),
        }
    }
    fn recorder(&self) -> Recorder {
        match self {
            Self::Register(_) => Recorder::Register(None),
            Self::Key(_) => Recorder::Key(None),
            Self::String(_) => Recorder::String(StringRecorder {
                string: "".into(),
                done: false,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // #[test]
    // fn im_0() {
    //     let vs = vec![
    //         ProcessingDescriptor::Key("space".into()),
    //         ProcessingDescriptor::Key("s".into()),
    //         ProcessingDescriptor::Key("i".into()),
    //         ProcessingDescriptor::Key("t".into()),
    //         ProcessingDescriptor::Register,
    //         ProcessingDescriptor::String,
    //     ];
    //     fn make_action(vs: &mut Vec<String>) -> CompleteAction {
    //         let string = vs.pop().unwrap();
    //         let register = vs.pop().unwrap();
    //         CompleteAction::BindRegisterToString(register, string)
    //     }
    //     let mut im = InputManager::new();
    //     im.bind(vs, make_action);
    //     let vs2 = vec![
    //         ProcessingDescriptor::Key("space".into()),
    //         ProcessingDescriptor::Key("s".into()),
    //         ProcessingDescriptor::Key("r".into()),
    //         ProcessingDescriptor::Key("r".into()),
    //         ProcessingDescriptor::Register,
    //         ProcessingDescriptor::Register,
    //     ];
    //     fn make_action_2(mut vs: Vec<String>) -> CompleteAction {
    //         let register_to_lookup = vs.pop().unwrap();
    //         let register_to_modify = vs.pop().unwrap();
    //         CompleteAction::SetRemove(register_to_modify, register_to_lookup)
    //     }
    //     im.bind(vs2, make_action_2);
    //     {
    //         let input_state = im.start();
    //         assert!(input_state.is_some());
    //         let input_state = input_state.unwrap();
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::Space,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::S,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::I,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::T,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::X,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::A,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(c) => {
    //                 panic!();
    //             }
    //         };
    //         let complete_action = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: true,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::Return,
    //             },
    //         ) {
    //             InputResult::InputState(_) => panic!(),
    //             InputResult::CompleteAction(c) => c,
    //         };
    //         assert_eq!(
    //             complete_action,
    //             CompleteAction::BindRegisterToString("x".into(), "a".into())
    //         );
    //     }

    //     // NEXT
    //     {
    //         let input_state = im.start();
    //         assert!(input_state.is_some());
    //         let input_state = input_state.unwrap();
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::Space,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::S,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::R,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::R,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let input_state = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::X,
    //             },
    //         ) {
    //             InputResult::InputState(s) => s,
    //             InputResult::CompleteAction(_) => panic!(),
    //         };
    //         let complete_action = match im.input(
    //             input_state,
    //             Input {
    //                 modifiers: &Modifiers {
    //                     shift_pressed: false,
    //                 },
    //                 virtual_key_code: &VirtualKeyCode::Y,
    //             },
    //         ) {
    //             InputResult::InputState(_) => panic!(),
    //             InputResult::CompleteAction(c) => c,
    //         };
    //         assert_eq!(
    //             complete_action,
    //             CompleteAction::SetRemove("x".into(), "y".into())
    //         );
    //     }
    // }
}
