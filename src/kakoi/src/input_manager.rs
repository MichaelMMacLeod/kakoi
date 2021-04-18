use crate::input_map::vk_to_keyname_string;
use crate::input_map::InputMap;
use crate::input_state::InputState;
use winit::event::VirtualKeyCode;
use winit::event::{ElementState, KeyboardInput};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum Action {
    SetRemove,
    InsertStringIntoSetRegister,
    SelectRegister,
    BindRegisterToRegisterValue,
    BindRegisterToString,
    Back,
    Registers,
}

pub enum CompleteAction {
    SetRemove(String, String),
    InsertStringIntoSetRegister(String, String),
    SelectRegister(String),
    BindRegisterToRegisterValue(String, String),
    BindRegisterToString(String, String),
    Registers,
    Back,
}

#[derive(Debug)]
struct ActionState {
    action: Action,
    recorder: Option<Recorder>,
    data: Vec<String>,
}

impl ActionState {
    fn new(action: Action) -> Self {
        Self {
            action,
            data: vec![],
            recorder: None,
        }
    }

    fn complete(&self) -> Option<CompleteAction> {
        match self.action {
            Action::SetRemove => match self.data.len() {
                2 => Some(CompleteAction::SetRemove(self.data[0].clone(), self.data[1].clone())),
                n if n > 2 => panic!(),
                _ => None,
            }
            Action::InsertStringIntoSetRegister => match self.data.len() {
                2 => Some(CompleteAction::InsertStringIntoSetRegister(self.data[0].clone(), self.data[1].clone())),
                n if n > 2 => panic!(),
                _ => None,
            }
            Action::SelectRegister => match self.data.len() {
                1 => Some(CompleteAction::SelectRegister(self.data[0].clone())),
                n if n > 1 => panic!(),
                _ => None,
            },
            Action::BindRegisterToRegisterValue => match self.data.len() {
                2 => Some(CompleteAction::BindRegisterToRegisterValue(
                    self.data[0].clone(),
                    self.data[1].clone(),
                )),
                n if n > 2 => panic!(),
                _ => None,
            },
            Action::BindRegisterToString => match self.data.len() {
                2 => Some(CompleteAction::BindRegisterToString(
                    self.data[0].clone(),
                    self.data[1].clone(),
                )),
                n if n > 2 => panic!(),
                _ => None,
            },
            Action::Back => match self.data.len() {
                0 => Some(CompleteAction::Back),
                _ => panic!(),
            },
            Action::Registers => match self.data.len() {
                0 => Some(CompleteAction::Registers),
                _ => panic!(),
            },
        }
    }

    fn record(&mut self, modifiers: &Modifiers, keyboard_input: &KeyboardInput) {
        if self.recorder.is_none() {
            self.recorder = Some(match self.action {
                Action::SetRemove => match self.data.len() {
                    0 => Recorder::Register,
                    1 => Recorder::Register,
                    _ => panic!(),
                }
                Action::InsertStringIntoSetRegister => match self.data.len() {
                    0 => Recorder::Register,
                    1 => Recorder::String("".into()),
                    _ => panic!(),
                }
                Action::SelectRegister => match self.data.len() {
                    0 => Recorder::Register,
                    _ => panic!(),
                },
                Action::BindRegisterToRegisterValue => match self.data.len() {
                    0 => Recorder::Register,
                    1 => Recorder::Register,
                    _ => panic!(),
                }
                Action::BindRegisterToString => match self.data.len() {
                    0 => Recorder::Register,
                    1 => Recorder::String("".into()),
                    _ => panic!(),
                },
                Action::Back => match self.data.len() {
                    _ => panic!(),
                },
                Action::Registers => match self.data.len() {
                    _ => panic!(),
                },
            });
        }

        match self
            .recorder
            .as_mut()
            .unwrap()
            .process(modifiers, keyboard_input)
        {
            Some(string) => {
                self.recorder = None;
                self.data.push(string);
            }
            None => {}
        }
    }
}

#[derive(Debug)]
enum Recorder {
    Register,
    String(String),
}

impl Recorder {
    fn process(&mut self, modifiers: &Modifiers, keyboard_input: &KeyboardInput) -> Option<String> {
        if keyboard_input.state == ElementState::Pressed {
            match self {
                Self::Register => {
                    Some(vk_to_keyname_string(keyboard_input.virtual_keycode?).into())
                }
                Self::String(string) => {
                    enum Do {
                        Insert(String),
                        Delete(bool),
                        Done,
                        Nothing,
                    }

                    let fix_case = |str: &'static str| -> String {
                        if modifiers.shift_pressed {
                            str.to_uppercase()
                        } else {
                            str.into()
                        }
                    };

                    let enter = || -> Do {
                        if modifiers.shift_pressed {
                            Do::Done
                        } else {
                            Do::Insert("\n".into())
                        }
                    };

                    let d = match keyboard_input.virtual_keycode? {
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
                        VirtualKeyCode::Delete => Do::Delete(modifiers.shift_pressed),
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
                            None
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
                            None
                        }
                        Do::Done => Some(string.clone()),
                        Do::Nothing => None,
                    }
                }
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct Modifiers {
    shift_pressed: bool,
}

impl Modifiers {
    fn new() -> Self {
        Self {
            shift_pressed: false,
        }
    }

    fn update(&mut self, keyboard_input: &KeyboardInput) {
        let pressed = keyboard_input.state == ElementState::Pressed;
        if let Some(virtual_key_code) = keyboard_input.virtual_keycode {
            match virtual_key_code {
                VirtualKeyCode::LShift | VirtualKeyCode::RShift => self.shift_pressed = pressed,
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
pub struct InputManager {
    input_state: InputState<Action>,
    modifiers: Modifiers,
    action_state: Option<ActionState>,
}

impl InputManager {
    pub fn new() -> Self {
        let mut input_map = InputMap::new();
        input_map.bind(vec!["space", "r", "s"], Action::SelectRegister);
        input_map.bind(vec!["space", "r", "b", "s"], Action::BindRegisterToString);
        input_map.bind(vec!["space", "r", "b", "r"], Action::BindRegisterToRegisterValue);
        input_map.bind(vec!["space", "b"], Action::Back);
        input_map.bind(vec!["space", "v"], Action::Registers);
        input_map.bind(vec!["space", "s", "i", "s"], Action::InsertStringIntoSetRegister);
        input_map.bind(vec!["space", "s", "r", "r"], Action::SetRemove);
        Self {
            input_state: InputState::new(input_map),
            modifiers: Modifiers::new(),
            action_state: None,
        }
    }

    pub fn input(&mut self, keyboard_input: &KeyboardInput) -> Option<CompleteAction> {
        self.modifiers.update(keyboard_input);
        dbg!(&self.input_state.current_input_sequence, &self.action_state);
        match &mut self.action_state {
            Some(action_state) => {
                action_state.record(&self.modifiers, keyboard_input);
                action_state.complete()
            }
            None => {
                if keyboard_input.state == ElementState::Pressed {
                    if let Some(virtual_key_code) = keyboard_input.virtual_keycode {
                        match self
                            .input_state
                            .input(vk_to_keyname_string(virtual_key_code))
                        {
                            Some(action) => {
                                self.action_state = Some(ActionState::new(*action));
                                self.action_state.as_ref().unwrap().complete()
                            }
                            None => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
        .map(|complete_action| {
            self.action_state = None;
            complete_action
        })
    }
}
