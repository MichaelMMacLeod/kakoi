use crate::forest::Forest;
use slotmap::new_key_type;
use winit::event::VirtualKeyCode;

new_key_type! {
    pub struct InputMapKey;
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct InputMapData<A> {
    input: String,
    action: Option<A>,
}

#[derive(Debug)]
pub struct InputMap<A> {
    forest: Forest<InputMapKey, InputMapData<A>>,
    root: InputMapKey,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Lookup<'a, A> {
    Complete(Result<&'a A, ()>),
    Incomplete,
}

impl<A> InputMap<A> {
    pub fn new() -> Self {
        let mut forest = Forest::new();
        let root = forest.insert_root(InputMapData {
            input: "".into(),
            action: None,
        });
        Self { forest, root }
    }

    pub fn bind<S: Into<String>>(&mut self, sequence: Vec<S>, action: A) {
        let mut previous = self.root;
        sequence.into_iter().for_each(|input| {
            let input = input.into();
            previous = {
                let children = self.forest.children(previous).unwrap().to_owned();
                children
                    .into_iter()
                    .find(|&s| match self.forest.get_mut(s).unwrap() {
                        InputMapData {
                            input: next_input,
                            action,
                        } => {
                            if input == *next_input {
                                *action = None;
                                true
                            } else {
                                false
                            }
                        }
                    })
                    .unwrap_or_else(|| {
                        self.forest.insert_child(
                            previous,
                            InputMapData {
                                input,
                                action: None,
                            },
                        )
                    })
            };
        });
        if previous != self.root {
            self.forest.get_mut(previous).unwrap().action = Some(action);
        }
    }

    pub fn lookup<S: Into<String> + Clone>(&self, sequence: &[S]) -> Lookup<A> {
        dbg!(sequence.iter().cloned().map(|s| s.into()).collect::<Vec<_>>());
        let mut previous = self.root;
        let sequence_length = sequence.len();

        let consumed_elements = sequence
            .iter()
            .cloned()
            .map(|s| s.into())
            .take_while(|input| {
                self.forest
                    .children(previous)
                    .unwrap()
                    .to_owned()
                    .into_iter()
                    .find(|&child_key| {
                        let InputMapData {
                            input: child_input, ..
                        } = self.forest.get(child_key).unwrap();
                        if child_input == input {
                            previous = child_key;
                            true
                        } else {
                            false
                        }
                    })
                    .is_some()
            })
            .count();

        if previous == self.root {
            if sequence_length == 0 {
                Lookup::Incomplete
            } else {
                Lookup::Complete(Result::Err(()))
            }
        } else {
            let InputMapData { action, .. } = self.forest.get(previous).unwrap();
            if consumed_elements == sequence_length {
                if let Some(action) = action.as_ref() {
                    Lookup::Complete(Result::Ok(action))
                } else {
                    Lookup::Incomplete
                }
            } else {
                Lookup::Complete(Result::Err(()))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn input_map_0() {
        #[derive(Debug, PartialEq, Eq)]
        enum Action {
            Open,
            Close,
        }
        let mut input_map = InputMap::new();
        input_map.bind(vec!["space", "o"], Action::Open);
        input_map.bind(vec!["space", "c"], Action::Close);

        assert_eq!(
            Lookup::Complete(Result::Ok(&Action::Open)),
            input_map.lookup(&["space", "o"])
        );
        assert_eq!(
            Lookup::Complete(Result::Ok(&Action::Close)),
            input_map.lookup(&["space", "c"])
        );
        assert_eq!(Lookup::Incomplete, input_map.lookup(&["space"]));
        assert_eq!(
            Lookup::Complete(Result::Err(())),
            input_map.lookup(&["x"])
        );

        input_map.bind(vec!["space", "o", "o"], Action::Open);
        input_map.bind(vec!["space", "o", "c"], Action::Close);

        assert_eq!(Lookup::Incomplete, input_map.lookup(&["space", "o"]));
        assert_eq!(
            Lookup::Complete(Result::Ok(&Action::Close)),
            input_map.lookup(&["space", "c"])
        );
        assert_eq!(
            Lookup::Complete(Result::Ok(&Action::Open)),
            input_map.lookup(&["space", "o", "o"])
        );
        assert_eq!(
            Lookup::Complete(Result::Ok(&Action::Close)),
            input_map.lookup(&["space", "o", "c"])
        );
    }
}

pub fn vk_to_keyname_string(code: &VirtualKeyCode) -> &'static str {
    match code {
        VirtualKeyCode::Key1 => "1",
        VirtualKeyCode::Key2 => "2",
        VirtualKeyCode::Key3 => "3",
        VirtualKeyCode::Key4 => "4",
        VirtualKeyCode::Key5 => "5",
        VirtualKeyCode::Key6 => "6",
        VirtualKeyCode::Key7 => "7",
        VirtualKeyCode::Key8 => "8",
        VirtualKeyCode::Key9 => "9",
        VirtualKeyCode::Key0 => "0",
        VirtualKeyCode::A => "a",
        VirtualKeyCode::B => "b",
        VirtualKeyCode::C => "c",
        VirtualKeyCode::D => "d",
        VirtualKeyCode::E => "e",
        VirtualKeyCode::F => "f",
        VirtualKeyCode::G => "g",
        VirtualKeyCode::H => "h",
        VirtualKeyCode::I => "i",
        VirtualKeyCode::J => "j",
        VirtualKeyCode::K => "k",
        VirtualKeyCode::L => "l",
        VirtualKeyCode::M => "m",
        VirtualKeyCode::N => "n",
        VirtualKeyCode::O => "o",
        VirtualKeyCode::P => "p",
        VirtualKeyCode::Q => "q",
        VirtualKeyCode::R => "r",
        VirtualKeyCode::S => "s",
        VirtualKeyCode::T => "t",
        VirtualKeyCode::U => "u",
        VirtualKeyCode::V => "v",
        VirtualKeyCode::W => "w",
        VirtualKeyCode::X => "x",
        VirtualKeyCode::Y => "y",
        VirtualKeyCode::Z => "z",
        VirtualKeyCode::Escape => "escape",
        VirtualKeyCode::F1 => "f1",
        VirtualKeyCode::F2 => "f2",
        VirtualKeyCode::F3 => "f3",
        VirtualKeyCode::F4 => "f4",
        VirtualKeyCode::F5 => "f5",
        VirtualKeyCode::F6 => "f6",
        VirtualKeyCode::F7 => "f7",
        VirtualKeyCode::F8 => "f8",
        VirtualKeyCode::F9 => "f9",
        VirtualKeyCode::F10 => "f10",
        VirtualKeyCode::F11 => "f11",
        VirtualKeyCode::F12 => "f12",
        VirtualKeyCode::F13 => "f13",
        VirtualKeyCode::F14 => "f14",
        VirtualKeyCode::F15 => "f15",
        VirtualKeyCode::F16 => "f16",
        VirtualKeyCode::F17 => "f17",
        VirtualKeyCode::F18 => "f18",
        VirtualKeyCode::F19 => "f19",
        VirtualKeyCode::F20 => "f20",
        VirtualKeyCode::F21 => "f21",
        VirtualKeyCode::F22 => "f22",
        VirtualKeyCode::F23 => "f23",
        VirtualKeyCode::F24 => "f24",
        VirtualKeyCode::Snapshot => "snapshot",
        VirtualKeyCode::Scroll => "scroll",
        VirtualKeyCode::Pause => "pause",
        VirtualKeyCode::Insert => "insert",
        VirtualKeyCode::Home => "home",
        VirtualKeyCode::Delete => "delete",
        VirtualKeyCode::End => "end",
        VirtualKeyCode::PageDown => "page_down",
        VirtualKeyCode::PageUp => "page_up",
        VirtualKeyCode::Left => "left",
        VirtualKeyCode::Up => "up",
        VirtualKeyCode::Right => "right",
        VirtualKeyCode::Down => "down",
        VirtualKeyCode::Back => "back",
        VirtualKeyCode::Return => "return",
        VirtualKeyCode::Space => "space",
        VirtualKeyCode::Compose => "compose",
        VirtualKeyCode::Caret => "caret",
        VirtualKeyCode::Numlock => "num_lock",
        VirtualKeyCode::Numpad0 => "0",
        VirtualKeyCode::Numpad1 => "1",
        VirtualKeyCode::Numpad2 => "2",
        VirtualKeyCode::Numpad3 => "3",
        VirtualKeyCode::Numpad4 => "4",
        VirtualKeyCode::Numpad5 => "5",
        VirtualKeyCode::Numpad6 => "6",
        VirtualKeyCode::Numpad7 => "7",
        VirtualKeyCode::Numpad8 => "8",
        VirtualKeyCode::Numpad9 => "9",
        VirtualKeyCode::Divide => "/",
        VirtualKeyCode::Decimal => ".",
        VirtualKeyCode::NumpadComma => ",",
        VirtualKeyCode::NumpadEnter => "enter",
        VirtualKeyCode::NumpadEquals => "=",
        VirtualKeyCode::Multiply => "*",
        VirtualKeyCode::Subtract => "-",
        VirtualKeyCode::AbntC1 => "abnt_c1",
        VirtualKeyCode::AbntC2 => "abnt_c2",
        VirtualKeyCode::Apostrophe => "'",
        VirtualKeyCode::Apps => "apps",
        VirtualKeyCode::At => "@",
        VirtualKeyCode::Ax => "ax",
        VirtualKeyCode::Backslash => "\\",
        VirtualKeyCode::Calculator => "calculator",
        VirtualKeyCode::Capital => "capital",
        VirtualKeyCode::Colon => ":",
        VirtualKeyCode::Comma => ",",
        VirtualKeyCode::Convert => "convert",
        VirtualKeyCode::Equals => "=",
        VirtualKeyCode::Grave => "`",
        VirtualKeyCode::Kana => "kana",
        VirtualKeyCode::Kanji => "kanji",
        VirtualKeyCode::LAlt => "alt",
        VirtualKeyCode::LBracket => "[",
        VirtualKeyCode::LControl => "control",
        VirtualKeyCode::LShift => "shift",
        VirtualKeyCode::LWin => "super",
        VirtualKeyCode::Mail => "mail",
        VirtualKeyCode::MediaSelect => "media_select",
        VirtualKeyCode::MediaStop => "media_stop",
        VirtualKeyCode::Minus => "-",
        VirtualKeyCode::Mute => "mute",
        VirtualKeyCode::MyComputer => "my_computer",
        VirtualKeyCode::NavigateForward => "navigate_forward",
        VirtualKeyCode::NavigateBackward => "navigate_backward",
        VirtualKeyCode::NextTrack => "next_track",
        VirtualKeyCode::NoConvert => "no_convert",
        VirtualKeyCode::OEM102 => "OEM102",
        VirtualKeyCode::Period => ".",
        VirtualKeyCode::PlayPause => "play_pause",
        VirtualKeyCode::Add => "+",
        VirtualKeyCode::Power => "power",
        VirtualKeyCode::PrevTrack => "previous_track",
        VirtualKeyCode::RAlt => "alt",
        VirtualKeyCode::RBracket => "]",
        VirtualKeyCode::RControl => "control",
        VirtualKeyCode::RShift => "shift",
        VirtualKeyCode::RWin => "super",
        VirtualKeyCode::Semicolon => ";",
        VirtualKeyCode::Slash => "/",
        VirtualKeyCode::Sleep => "sleep",
        VirtualKeyCode::Stop => "stop",
        VirtualKeyCode::Sysrq => "sysrq",
        VirtualKeyCode::Tab => "tab",
        VirtualKeyCode::Underline => "_",
        VirtualKeyCode::Unlabeled => "unlabeled",
        VirtualKeyCode::VolumeDown => "volume_down",
        VirtualKeyCode::VolumeUp => "volume_up",
        VirtualKeyCode::Wake => "wake",
        VirtualKeyCode::WebBack => "web_back",
        VirtualKeyCode::WebFavorites => "web_favorites",
        VirtualKeyCode::WebForward => "web_forward",
        VirtualKeyCode::WebHome => "web_home",
        VirtualKeyCode::WebRefresh => "web_refresh",
        VirtualKeyCode::WebSearch => "web_search",
        VirtualKeyCode::WebStop => "web_stop",
        VirtualKeyCode::Yen => "yen",
        VirtualKeyCode::Copy => "copy",
        VirtualKeyCode::Paste => "paste",
        VirtualKeyCode::Cut => "cut",
    }
}

// struct Input {
//     virtual_key_code: VirtualKeyCode,
// }

// fn modify_string(string: &mut String, input: winit::event::KeyboardInput) {
//     let winit::event::KeyboardInput {
//         state,
//         virtual_keycode,
//         ..
//     } = input;
    
// }