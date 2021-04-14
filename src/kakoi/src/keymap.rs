use std::collections::HashMap;

use winit::event::VirtualKeyCode;

struct KeySequence(String);

struct RawKeySequence {
    raw_sequence: Vec<VirtualKeyCode>,
}

impl RawKeySequence {
    fn to_key_sequence(&self) -> KeySequence {
        KeySequence(self.raw_sequence
            .iter()
            .zip(0..)
            .map(|(v, i)| {
                if i == self.raw_sequence.len() - 1 {
                    format!("{}", vk_to_string(*v))
                } else {
                    format!("{} ", vk_to_string(*v))
                }
            })
            .collect())

        // `intersperse' is, as of 2021-04-14, nightly-only. When it gets 
        // stabilized, we can use it instead.
    }
}

struct InputState {
    current_sequence: KeySequence,
    keymap: KeyMap, 
}

struct KeyMap {
    map: HashMap<KeySequence, fn()>
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn key_sequence_string_1() {
        assert_eq!(
            "space a o r f",
            RawKeySequence {
                raw_sequence: vec![
                    VirtualKeyCode::Space,
                    VirtualKeyCode::A,
                    VirtualKeyCode::O,
                    VirtualKeyCode::R,
                    VirtualKeyCode::F
                ]
            }.to_key_sequence().0
        );
    }
}

fn vk_to_string(code: VirtualKeyCode) -> &'static str {
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
        VirtualKeyCode::Key0 => "10",
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
