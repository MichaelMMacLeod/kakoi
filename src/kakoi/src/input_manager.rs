//! # User input handling
//!
//! This module exports [`InputManager`], a struct that handles keyboard input
//! processing. It contains one method,
//! [`process_input`](InputManager::process_input), that receive a keyboard
//! event from the window and (maybe) returns a [`CompleteAction`].
//!
//! At any point in time the user is expected to enter a certain type of input.
//! Most commonly, we expect the user to press a single key. It is also possible
//! to expect different things. For instance, when inserting a piece of text the
//! user is instead expected to make zero-or-more key presses followed by a
//! press of shift+enter to complete the string. The different expectations of
//! what types of things the user should enter are encapsulated by
//! [`InputRequirementDescriptor`]s.
//!
//! A key binding is described as a sequence of [`InputRequirementDescriptor`]s
//! followed by a function called an action constructor. The action constructor
//! receives the input the user has entered and produces a [`CompleteAction`].
//! The [`CompleteAction`] may contain elements of what the user entered. Use
//! the [`key`], [`register`], and [`string`] functions as shorthands for
//! creating [`InputRequirementDescriptor`]s.
//!
//! Key bindings are stored in a [`KeyBinder`]. The [`KeyBinder`] can be thought
//! of as a [finite state machine] where each state represents a certain stage
//! of user input. Each stage has associated with it a type which corresponds to
//! the kind of input we expect the user to enter from that stage (see
//! [`InputRequirementDescriptor`]).
//!
//! The [`KeyBinder`] does not contain any data about the current state of user
//! input. That state is instead encapsulated inside [`InputState`]s. The
//! initial input state can be produced by [`KeyBinder::start_state`]. The
//! current [`InputState`] is contained within the [`InputManager`].
//!
//! [finite state machine]: https://en.wikipedia.org/wiki/Finite-state_machine

use slotmap::{new_key_type, SlotMap};
use std::collections::{HashMap, VecDeque};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

/// Describes a command the user has inputted.
///
/// These actions are produced by the functions passed in as arguments to
/// [`KeyBinder::bind`].
///
/// Each [`CompleteAction`] may have associated with it some data, probably in
/// the form of [`String`]s. This data may come from the user, but it may also
/// come from the function that created the [`CompleteAction`].
///
/// Whenever a [`CompleteAction`] is meant to modify something (the contents of
/// a register, for instance), the argument that gets modified comes first.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CompleteAction {
    /// SetInsert(register_a, register_b)
    ///
    /// Inserts the value bound to register_b into the set bound to register_a.
    SetInsert(String, String),
    /// SetUnion(register_a, register_b)
    ///
    /// Modifies the set bound to register_a so that it includes all of the
    /// values inside the set bound to register_a as well as all of the values
    /// inside the set bound to register_b.
    SetUnion(String, String),
    /// SetRemove(register_a, register_b)
    ///
    /// Removes the value bound to register_b from the set bound to register_a.
    SetRemove(String, String),
    /// InsertStringIntoSetRegister(register, string)
    ///
    /// Inserts `string` into the set bound to a register.
    InsertStringIntoSetRegister(String, String),
    /// SelectRegister(register_to_focus)
    ///
    /// Binds the register `.` to the value stored in register_to_focus. This
    /// has the consequence of making the value bound in register_to_focus be
    /// displayed on screen, since `.` is the register containing the value
    /// currently displayed on screen.
    SelectRegister(String),
    /// BindRegisterToRegisterValue(to_modify, to_lookup)
    ///
    /// Binds the register to_modify to the value bound to to_lookup.
    BindRegisterToRegisterValue(String, String),
    /// BindRegisterToString(register, string)
    ///
    /// Binds a register to a string.
    BindRegisterToString(String, String),
    /// BindRegisterToEmptySet(register)
    ///
    /// Binds a register to an empty set.
    BindRegisterToEmptySet(String),
    /// Registers
    ///
    /// Binds the register `.` to the map of register-value bindings.
    Registers,
    /// Back
    ///
    /// Binds the register `.` to the previously-focused value.
    Back,
}

/// Encapsulates everything needed to process user keyboard input.
///
/// See [the module-level documentation](crate::input_manager) for more
/// information.
pub struct InputManager {
    key_binder: KeyBinder,
    input_state: InputState,
    pressed_keys: PressedKeys,
}

impl InputManager {
    /// Creates a new [`InputManager`] backed with the default key bindings.
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

    /// Receives keyboard input from the window, accumulating it. Returns a
    /// [`CompleteAction`] if the current accumulation of input is complete,
    /// as is determined by the [`KeyBinder`].
    pub fn process_input(&mut self, keyboard_input: &KeyboardInput) -> Option<CompleteAction> {
        let pressed = keyboard_input.state == ElementState::Pressed;
        if let Some(virtual_key_code) = &keyboard_input.virtual_keycode {
            // Update `modifiers` if shift was pressed or unpressed.
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

            // key_binder.process_input is meant to be called on key presses,
            // not key releases. This might change in the future if releasing a
            // key becomes something that can be tracked inside of the key
            // binder.
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

/// A stage of user input inside a [`KeyBinder`].
///
/// These stages can be referred to using [`KeyBinderKey`]s.
enum InputAccumulationStage {
    /// A stage that expects the user to input something of a certain type.
    /// Contains information about which [`InputAccumulationStage`]s to go to
    /// next, possibly depending on what the user entered.
    InputRequirement(InputRequirement),
    /// The final stage of a series of user inputs. Holds a function that takes
    /// the user input (as was accumulated in previous stages) and produces a
    /// [`CompleteAction`].
    Done(fn(accumulated_input: &mut Vec<String>) -> CompleteAction),
}

/// Associates descriptions of user input with [`CompleteAction`]s.
struct KeyBinder {
    /// The underlying storage container.
    slot_map: SlotMap<KeyBinderKey, InputAccumulationStage>,
    /// The first stage of user input. If `KeyBinder::bind` has never been
    /// called, this is [`None`].
    start_stage: Option<KeyBinderKey>,
}

impl KeyBinder {
    /// Creates a new [`KeyBinder`] with no key bindings.
    fn new() -> Self {
        Self {
            slot_map: SlotMap::with_key(),
            start_stage: None,
        }
    }

    /// Registers the 'default' key bindings.
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

    /// Associates a description of user input with a function that takes that
    /// input and returns a [`CompleteAction`].
    ///
    /// This function will panic if you attempt to overwrite an existing
    /// keybinding. For example, binding [key(w), key(f), string()] AND [key(w),
    /// key(f), register()] will panic. This extremely-panicky behavior should
    /// probably be changed.
    ///
    /// Note: this function will NOT panic if you add overlapping keybindings of
    /// the same type. For instance, binding [key(w), key(s), string()] AND
    /// [key(w), key(r), register()] will not panic, since key(s) and key(r) are
    /// both of the same type (expecting the user to input a key).
    fn bind(
        &mut self,
        descriptors: Vec<InputRequirementDescriptor>,
        action_constructor: fn(accumulated_input: &mut Vec<String>) -> CompleteAction,
    ) {
        let action_key = self
            .slot_map
            .insert(InputAccumulationStage::Done(action_constructor));
        // Insert the descriptors in the reverse order. We need to do this in
        // reverse because each InputRequirement needs to know the KeyBinderKey
        // of the NEXT InputRequirement; key(a) in [key(a), key(b)] needs to
        // know that when you press 'a', you need to transition into the key(b)
        // stage, which we refer to using key(b)'s KeyBinderKey. The only way to
        // get key(b)'s KeyBinderKey is to insert it into the slot_map, so we
        // have to insert key(b) before inserting key(a).
        let first_key = descriptors.into_iter().rev().fold(
            action_key,
            |next_key, input_requirement_descriptor| {
                self.slot_map
                    .insert(InputAccumulationStage::InputRequirement(
                        input_requirement_descriptor.to_input_requirement(next_key),
                    ))
            },
        );
        match self.start_stage {
            // If this is the first time this function was called, simply mark
            // first_key as the starting stage.
            None => self.start_stage = Some(first_key),
            // Otherwise, there already is a starting stage. We need to attempt
            // to merge the new sequence of input descriptions associated with
            // first_key into the sequence of input descriptions already
            // associated with start_stage.
            //
            // If successful, this 'merge' will change the data associated with
            // start_stage to include the data associated with first_key. Any
            // redundant stages in first_key (which, as a result of the merging
            // process, now have copies in start_stage) will be removed from the
            // slot_map.
            Some(start_state) => {
                recursively_merge(
                    &mut self.slot_map,
                    &mut vec![Merge {
                        // we are modifying start_stage to contain the
                        // keybinding information from first_key. In other
                        // words, we are merging FROM first_key INTO
                        // start_stage.
                        into: start_state,
                        from: first_key,
                    }]
                    .into_iter()
                    .collect(),
                );
            }
        }
    }

    /// Returns the [`InputState`] representing a 'starting point' for user
    /// input, or [`None`] if no keybindings have been registered.
    fn start_state(&self) -> Option<InputState> {
        self.start_stage.map(|start_state| InputState {
            current_stage: start_state,
            processed_input: vec![],
            current_processor: None,
        })
    }

    /// Modifies `input_state` based on `input`, possibly returning a
    /// [`CompleteAction`] if `input` moves `input_state` into a
    /// [`InputAccumulationStage::Done`] stage.
    ///
    /// This function will panic with [`unreachable!`] if `input_state` is
    /// already in a [`InputAccumulationStage::Done`] at the time of calling.
    /// Since [`InputState`]s should only ever be created through
    /// [`KeyBinder::start_state`], and since they should only ever be modified
    /// through this function, it should be impossible for this to happen
    /// (assuming that [`KeyBinder::bind`] binds keys correctly).
    fn process_input<'a>(
        &self,
        input_state: &mut InputState,
        input: Input<'a>,
    ) -> Option<CompleteAction> {
        match self.slot_map.get(input_state.current_stage).unwrap() {
            InputAccumulationStage::Done(_) => unreachable!(),
            InputAccumulationStage::InputRequirement(processing) => {
                // if this is the first time process_input has been called with
                // input_state in its current stage, it does not yet have a
                // processor. Create it.
                let processor = input_state
                    .current_processor
                    .get_or_insert_with(|| processing.processor());
                // we can't modify `input_state` inside of the closure receiving
                // processor.process(input), so we store what we want to modify
                // here in `next_value` and `next_stage`, then modify
                // `input_state` later.
                let mut next_value = None;
                let mut next_stage = None;
                processor.process(input).map(|result| {
                    next_stage = Some(processing.next_stage(&result)).unwrap();
                    next_value = Some(result);
                });
                // (later):
                next_value.map(|next_value| {
                    input_state.processed_input.push(next_value);
                    input_state.current_processor = None;
                });
                next_stage.map(|next_stage| input_state.current_stage = *next_stage);
                if let InputAccumulationStage::Done(action_constructor) =
                    self.slot_map.get(input_state.current_stage).unwrap()
                {
                    let complete_action = action_constructor(&mut input_state.processed_input);

                    // return `input_state` to the start state, so its memory
                    // can be re-used for handling future inputs.

                    input_state.processed_input.clear();
                    // if we don't reset to a non-Done stage, we could hit the
                    // unreachable! code above.
                    input_state.current_stage = self.start_stage.unwrap();
                    input_state.current_processor = None;
                    Some(complete_action)
                } else {
                    None
                }
            }
        }
    }
}

/// Encapsulates a stage of input and the previously-accumulated input.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InputState {
    /// A description of the current stage of input.
    current_stage: KeyBinderKey,
    /// The [`InputProcessor`] corresponding to `current_stage`. Processes
    /// different types of input based on the type of `current_stage`.
    current_processor: Option<InputProcessor>,
    /// The input that has been accumulated from previous stages. This does not
    /// contain the input being currently accumulated in the current stage
    /// (that's a part of `current_processor`).
    processed_input: Vec<String>,
}

new_key_type! {
    /// Key for referring to [`InputAccumulationStage`]s in a [`KeyBinder`].
    pub struct KeyBinderKey;
}

/// Shorthand for `InputRequirementDescriptor::Key(str.to_owned())`.
pub fn key(str: &str) -> InputRequirementDescriptor {
    InputRequirementDescriptor::Key(str.to_owned())
}

/// Shorthand for `InputRequirementDescriptor::String`.
pub fn string() -> InputRequirementDescriptor {
    InputRequirementDescriptor::String
}

/// Shorthand for `InputRequirementDescriptor::Register`.
pub fn register() -> InputRequirementDescriptor {
    InputRequirementDescriptor::Register
}

/// Describes the type of input we expect to receive in a current stage.
///
/// Use the [`key`], [`string`], and [`register`] functions to create these less
/// verbosely.
pub enum InputRequirementDescriptor {
    /// We expect the user to press a specific key. The key we expect the user
    /// to press is the [`String`].
    Key(String),
    /// We expect the user to enter a register. Entering a register is the same
    /// as entering a Key, except that we move on to the next stage regardless
    /// of which register was entered.
    Register,
    /// We expect the user to enter a string. Strings are entered by pressing a
    /// series of keys (the characters of the string) followed by shift+enter to
    /// terminate the string. We move onto the next stage regardless of what
    /// string the user entered.
    ///
    /// In the future, it may be a good idea to add another variant to this enum
    /// that can move onto different stages based on the properties of the
    /// string we entered. For instance, maybe we move onto a different state
    /// depending on whether or not the string entered matches a regular
    /// expression.
    String,
}

impl InputRequirementDescriptor {
    /// Transforms this [`InputRequirementDescriptor`] into an
    /// [`InputRequirement`] suitable for storing in a [`KeyMap`]. The returned
    /// value contains information about when to transition into the next stage
    /// of input.
    ///
    /// Arguments:
    /// 
    /// * `key`: the next stage of input.
    fn to_input_requirement(self, key: KeyBinderKey) -> InputRequirement {
        match self {
            Self::Register => InputRequirement::Register(key),
            Self::String => InputRequirement::String(key),
            Self::Key(code) => InputRequirement::Key(vec![(code, key)].into_iter().collect()),
        }
    }
}

/// A description of an input stage's type, as well as the method for selecting
/// the next stage of input.
enum InputRequirement {
    /// The user will input a register name. We unconditionally move onto a
    /// single next stage, represented by the [`KeyBinderKey`].
    Register(KeyBinderKey),
    /// The user will input an entire [`String`]. We unconditionally move onto a
    /// single next stage, represented by the [`KeyBinderKey`].
    String(KeyBinderKey),
    /// The user will input a single key. To determine what stage to go to next,
    /// we look up they key in the [`HashMap`].
    Key(HashMap<String, KeyBinderKey>),
}

/// Accumulates a string, marking whether or not the user is done creating it.
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
    into: KeyBinderKey,
    from: KeyBinderKey,
}

fn recursively_merge(
    slot_map: &mut SlotMap<KeyBinderKey, InputAccumulationStage>,
    todo: &mut VecDeque<Merge>,
) {
    use InputAccumulationStage::InputRequirement as IR;
    use InputRequirement::Key as IK;
    while let Some(Merge { into, from }) = todo.pop_front() {
        match slot_map.get_disjoint_mut([into, from]).unwrap() {
            [IR(IK(into_map)), IR(IK(from_map))] => {
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
    fn next_stage(&self, data: &String) -> Option<&KeyBinderKey> {
        match self {
            InputRequirement::Register(k) => Some(k),
            InputRequirement::String(k) => Some(k),
            InputRequirement::Key(map) => map.get(data),
        }
    }
    fn processor(&self) -> InputProcessor {
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
