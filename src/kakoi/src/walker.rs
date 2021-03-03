use crate::action::Action;
use crate::index::Index;

#[derive(PartialEq, Eq, Debug)]
enum Instruction<S, I>
where
    S: Copy,
    I: Index,
{
    Indicate(S),
    IndicateCopy(I, S),
    Extend(S),
}

struct CopyInstructor<'a, 'b, I, S, SI>
where
    I: Index,
    S: 'b + Copy,
    SI: Iterator<Item = &'b (S, S)>,
{
    current: I,
    source: SI,
    actions: &'a [Action<I, S>],
    done: bool,
}

impl<'a, 'b, I, S, SI> CopyInstructor<'a, 'b, I, S, SI>
where
    I: Index,
    S: 'b + Copy,
    SI: Iterator<Item = &'b (S, S)>,
{
    pub fn new(start: I, source: SI, actions: &'a [Action<I, S>]) -> Self {
        Self {
            current: start,
            source,
            actions,
            done: false,
        }
    }
}

impl<'a, 'b, I, S, SI> CopyInstructor<'a, 'b, I, S, SI>
where
    I: Index,
    S: 'b + Copy,
    SI: Iterator<Item = &'b (S, S)>,
{
    // to fix this, try making CopyInstructor keep track of two different indices:
    // - an index tracking the position in source
    // - an index tracking the position in the copy
    fn process_action(&mut self) -> Option<Instruction<S, I>> {
        loop {
            match self.actions.split_first() {
                Some((first, rest)) => {
                    let i = match first {
                        Action::Insert(i, _) => i,
                        Action::Remove(i) => i,
                    };

                    if self.current.indicates(i) {
                        break;
                    } else {
                        self.actions = rest;
                    }
                }
                None => break,
            }
        }

        let current = self.current.clone();

        let instruction = match self.actions.first() {
            Some(action) => match action {
                Action::Insert(index, object) => {
                    if self.current.directly_indicates(index) {
                        self.current.reduce_mut();
                        Some(Instruction::Indicate(*object))
                    } else if self.current.indirectly_indicates(index) {
                        match self.source.next() {
                            Some((_, to)) => {
                                self.current.reduce_mut();
                                Some(Instruction::IndicateCopy(current, *to))
                            }
                            None => todo!(), // probably panic here
                        }
                    } else {
                        match self.source.next() {
                            Some((_, to)) => {
                                self.current.reduce_mut();
                                Some(Instruction::Indicate(*to))
                            }
                            None => None,
                        }
                    }
                }
                Action::Remove(index) => {
                    if self.current.directly_indicates(index) {
                        self.source.next();
                        self.actions = &self.actions[1..];

                        None
                    } else if self.current.indirectly_indicates(index) {
                        match self.source.next() {
                            Some((_, to)) => {
                                self.current.reduce_mut();
                                Some(Instruction::IndicateCopy(current, *to))
                            }
                            None => todo!(), // probably panic here
                        }
                    } else {
                        match self.source.next() {
                            Some((_, to)) => {
                                self.current.reduce_mut();
                                Some(Instruction::Indicate(*to))
                            }
                            None => todo!(), // probably panic here
                        }
                    }
                }
            },
            None => {
                if self.done {
                    None
                } else {
                    self.done = true;

                    match self.source.next() {
                        Some((from, _)) => Some(Instruction::Extend(*from)),
                        None => None,
                    }
                }
            }
        };

        // match &instruction {
        //     Some(i) => match i {
        //         Instruction::Indicate(_) => self.current.reduce_mut(),
        //         Instruction::IndicateCopy(_, _) => self.current.reduce_mut(),
        //         Instruction::Extend(_) => {}
        //     },
        //     None => self.current.reduce_mut(),
        // }

        instruction
    }
}

impl<'a, 'b, I, S, SI> Iterator for CopyInstructor<'a, 'b, I, S, SI>
where
    I: Index,
    S: 'b + Copy,
    SI: Iterator<Item = &'b (S, S)>,
{
    type Item = Instruction<S, I>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut instruction = None;
        let mut processed_len_0 = false;

        while instruction.is_none() && !processed_len_0 {
            let no_actions_before = self.actions.len() == 0;

            instruction = self.process_action();

            if instruction.is_none() && no_actions_before {
                processed_len_0 = true;
            }
        }

        instruction
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitvec::prelude::*;

    #[test]
    fn copy_instructor_0() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Insert(bitvec![0, 1], 6)][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::Indicate(6)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_1() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Insert(bitvec![0, 1, 1], 6)][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::IndicateCopy(bitvec![0], 2)), ci.next());
        assert_eq!(Some(Instruction::Extend(1)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_2() {
        let start = bitvec![0];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Insert(bitvec![0, 1, 1], 6)][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::IndicateCopy(bitvec![0], 4)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_3() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Insert(bitvec![0, 0, 0, 1], 6)][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::Indicate(2)), ci.next());
        assert_eq!(Some(Instruction::Indicate(0)), ci.next());
        assert_eq!(Some(Instruction::Indicate(6)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_4() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Remove(bitvec![1])][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_5() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Remove(bitvec![0, 1])][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::Extend(1)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_6() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Remove(bitvec![0, 0, 1])][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::Indicate(2)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_7() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Remove(bitvec![1, 1])][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::IndicateCopy(bitvec![], 4)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_8() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Remove(bitvec![0, 1, 1])][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::IndicateCopy(bitvec![0], 2)), ci.next());
        assert_eq!(Some(Instruction::Extend(1)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_9() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[
            Action::Remove(bitvec![1]),
            Action::Remove(bitvec![0, 1]),
            Action::Remove(bitvec![0, 0, 1]),
        ][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_10() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Remove(bitvec![1]), Action::Remove(bitvec![0, 1])][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Extend(1)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_11() {
        let start = bitvec![];
        let source = [(5, 4), (3, 2), (1, 0)].iter();
        let actions = &[Action::Remove(bitvec![1]), Action::Insert(bitvec![1], 6)][..];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(6)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }
}
