use crate::action::Action;
use crate::index::Index;
use std::iter::Peekable;

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

struct CopyInstructor<'a, 'b, I, S, SI, AI>
where
    I: 'a + Index,
    S: 'a + 'b + Copy,
    SI: IntoIterator<Item = &'b (S, S)>,
    AI: IntoIterator<Item = &'a Action<I, S>>,
{
    current_source: I,
    current_copy: I,
    source: SI::IntoIter,
    actions: Peekable<AI::IntoIter>,
    done: bool,
}

impl<'a, 'b, I, S, SI, AI> CopyInstructor<'a, 'b, I, S, SI, AI>
where
    I: 'a + Index,
    S: 'a + 'b + Copy,
    SI: IntoIterator<Item = &'b (S, S)>,
    AI: IntoIterator<Item = &'a Action<I, S>>,
{
    pub fn new(start: I, source: SI, actions: AI) -> Self {
        Self {
            current_source: start.clone(),
            current_copy: start,
            source: source.into_iter(),
            actions: actions.into_iter().peekable(),
            done: false,
        }
    }
}

enum Status<S, I>
where
    S: Copy,
    I: Index,
{
    Done,
    NotDone,
    Processed(Instruction<S, I>),
}

impl<S, I> Status<S, I>
where
    I: Index,
    S: Copy,
{
    fn is_not_done(&self) -> bool {
        match self {
            Status::NotDone => true,
            _ => false,
        }
    }
}

impl<'a, 'b, I, S, SI, AI> CopyInstructor<'a, 'b, I, S, SI, AI>
where
    I: 'a + Index,
    S: 'a + 'b + Copy,
    SI: IntoIterator<Item = &'b (S, S)>,
    AI: IntoIterator<Item = &'a Action<I, S>>,
{
    fn process_action(&mut self) -> Status<S, I> {
        let current_copy = &self.current_copy;
        let action = loop {
            let action = self.actions.peek();

            match action {
                Some(a) => {
                    if !current_copy.indicates(a.index()) {
                        self.actions.next();
                    } else {
                        break Some(a);
                    }
                }
                None => break None,
            }
        };

        let instruction = match action {
            Some(action) => match action {
                Action::Insert(index, object) => {
                    if self.current_copy.directly_indicates(index) {
                        self.current_source.reduce_mut();
                        self.current_copy.reduce_mut();
                        self.actions.next();
                        Status::Processed(Instruction::Indicate(*object))
                    } else if self.current_copy.indirectly_indicates(index) {
                        match self.source.next() {
                            Some((_, to)) => {
                                let c_current_source = self.current_source.clone();
                                self.current_source.reduce_mut();
                                self.current_copy.reduce_mut();
                                self.actions.next();
                                Status::Processed(Instruction::IndicateCopy(c_current_source, *to))
                            }
                            None => todo!(), // probably panic here
                        }
                    } else {
                        match self.source.next() {
                            Some((_, to)) => {
                                self.current_source.reduce_mut();
                                self.current_copy.reduce_mut();
                                Status::Processed(Instruction::Indicate(*to))
                            }
                            None => Status::Done,
                        }
                    }
                }
                Action::Remove(index) => {
                    if self.current_source.directly_indicates(index) {
                        self.source.next();
                        self.current_source.reduce_mut();
                        self.actions.next();
                        // self.actions = &self.actions[1..];

                        Status::NotDone
                    } else if self.current_source.indirectly_indicates(index) {
                        match self.source.next() {
                            Some((_, to)) => {
                                let c_current_source = self.current_source.clone();
                                self.current_source.reduce_mut();
                                self.current_copy.reduce_mut();
                                self.actions.next();
                                Status::Processed(Instruction::IndicateCopy(c_current_source, *to))
                            }
                            None => todo!(), // probably panic here
                        }
                    } else {
                        match self.source.next() {
                            Some((_, to)) => {
                                self.current_source.reduce_mut();
                                self.current_copy.reduce_mut();
                                Status::Processed(Instruction::Indicate(*to))
                            }
                            None => todo!(), // probably panic here
                        }
                    }
                }
            },
            None => {
                if self.done {
                    Status::Done
                } else {
                    self.done = true;

                    match self.source.next() {
                        Some((from, _)) => Status::Processed(Instruction::Extend(*from)),
                        None => Status::Done,
                    }
                }
            }
        };

        instruction
    }
}

impl<'a, 'b, I, S, SI, AI> Iterator for CopyInstructor<'a, 'b, I, S, SI, AI>
where
    I: 'a + Index,
    S: 'a + 'b + Copy,
    SI: IntoIterator<Item = &'b (S, S)>,
    AI: IntoIterator<Item = &'a Action<I, S>>,
{
    type Item = Instruction<S, I>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut instruction = Status::NotDone;

        while instruction.is_not_done() {
            instruction = self.process_action();
        }

        match instruction {
            Status::NotDone => panic!("unreachable state"),
            Status::Done => None,
            Status::Processed(i) => Some(i),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitvec::prelude::*;

    #[test]
    fn copy_instructor_0() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Insert(bitvec![0, 1], 6)];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::Indicate(6)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_1() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Insert(bitvec![0, 1, 1], 6)];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::IndicateCopy(bitvec![0], 2)), ci.next());
        assert_eq!(Some(Instruction::Extend(1)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_2() {
        let start = bitvec![0];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Insert(bitvec![0, 1, 1], 6)];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::IndicateCopy(bitvec![0], 4)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_3() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Insert(bitvec![0, 0, 0, 1], 6)];
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
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Remove(bitvec![1])];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_5() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Remove(bitvec![0, 1])];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::Extend(1)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_6() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Remove(bitvec![0, 0, 1])];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::Indicate(2)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_7() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Remove(bitvec![1, 1])];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::IndicateCopy(bitvec![], 4)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_8() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Remove(bitvec![0, 1, 1])];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(4)), ci.next());
        assert_eq!(Some(Instruction::IndicateCopy(bitvec![0], 2)), ci.next());
        assert_eq!(Some(Instruction::Extend(1)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_9() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[
            Action::Remove(bitvec![1]),
            Action::Remove(bitvec![0, 1]),
            Action::Remove(bitvec![0, 0, 1]),
        ];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_10() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Remove(bitvec![1]), Action::Remove(bitvec![0, 1])];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Extend(1)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_11() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[Action::Remove(bitvec![1]), Action::Insert(bitvec![1], 6)];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(6)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_12() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[
            Action::Remove(bitvec![1, 1]),
            Action::Insert(bitvec![1, 1], 6),
        ];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::IndicateCopy(bitvec![], 4)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_13() {
        let start = bitvec![0];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[
            Action::Remove(bitvec![1, 1]),
            Action::Insert(bitvec![1, 1], 6),
        ];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Extend(5)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_14() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[
            Action::Insert(bitvec![1], 6),
            Action::Insert(bitvec![0, 1], 7),
        ];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(6)), ci.next());
        assert_eq!(Some(Instruction::Indicate(7)), ci.next());
        assert_eq!(Some(Instruction::Extend(5)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_15() {
        let start = bitvec![0];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[
            Action::Insert(bitvec![1], 6),
            Action::Insert(bitvec![0, 1], 7),
        ];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(7)), ci.next());
        assert_eq!(Some(Instruction::Extend(5)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_16() {
        let start = bitvec![];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[
            Action::Remove(bitvec![1]),
            Action::Remove(bitvec![0, 1]),
            Action::Remove(bitvec![0, 0, 1]),
            Action::Insert(bitvec![1], 6),
            Action::Insert(bitvec![0, 1], 7),
            Action::Insert(bitvec![0, 0, 1], 8),
        ];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(6)), ci.next());
        assert_eq!(Some(Instruction::Indicate(7)), ci.next());
        assert_eq!(Some(Instruction::Indicate(8)), ci.next());
        assert_eq!(None, ci.next());
    }

    #[test]
    fn copy_instructor_17() {
        let start = bitvec![0, 0];
        let source = &[(5, 4), (3, 2), (1, 0)];
        let actions = &[
            Action::Remove(bitvec![1]),
            Action::Remove(bitvec![0, 1]),
            Action::Remove(bitvec![0, 0, 1]),
            Action::Insert(bitvec![1], 6),
            Action::Insert(bitvec![0, 1], 7),
            Action::Insert(bitvec![0, 0, 1], 8),
        ];
        let mut ci = CopyInstructor::new(start, source, actions);
        assert_eq!(Some(Instruction::Indicate(8)), ci.next());
        assert_eq!(Some(Instruction::Extend(3)), ci.next());
        assert_eq!(None, ci.next());
    }
}
