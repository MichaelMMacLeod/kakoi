use crate::input_map::{InputMap, Lookup};

pub struct InputState<A> {
    input_map: InputMap<A>,
    current_input_sequence: Vec<String>,
}

impl<A> InputState<A> {
    pub fn new(input_map: InputMap<A>) -> Self {
        Self {
            input_map,
            current_input_sequence: vec![],
        }
    }

    pub fn input<S: Into<String>>(&mut self, input: S) -> Option<&A> {
        self.current_input_sequence.push(input.into());
        match self.input_map.lookup(&self.current_input_sequence) {
            Lookup::Complete(result) => {
                self.current_input_sequence.clear();
                result.ok()
            },
            Lookup::Incomplete => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn input_state_1() {
        #[derive(Debug, PartialEq, Eq, Clone)]
        enum Action {
            Open,
            Close,
            MoveDown,
        }
        let mut input_map = InputMap::new();
        input_map.bind(vec!["space", "o"], Action::Open);
        input_map.bind(vec!["space", "c"], Action::Close);
        input_map.bind(vec!["j"], Action::MoveDown);
        let mut input_state = InputState::new(input_map);
        assert!(input_state.input("space").is_none());
        assert_eq!(input_state.input("o"), Some(&Action::Open));
        assert!(input_state.input("space").is_none());
        assert_eq!(input_state.input("c"), Some(&Action::Close));
        assert!(input_state.input("space").is_none());
        assert!(input_state.input("space").is_none());
        assert!(input_state.input("space").is_none());
        assert_eq!(input_state.input("o"), Some(&Action::Open));
        assert_eq!(input_state.input("j"), Some(&Action::MoveDown));
    }
}