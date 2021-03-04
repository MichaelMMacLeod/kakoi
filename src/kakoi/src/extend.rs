use crate::index::Index;
use bitvec::prelude::*;
use std::collections::VecDeque;

#[derive(Debug, Eq, PartialEq, Clone)]
struct Insert<I: Index, T: Clone> {
    at: I,
    object: T,
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct Remove<I: Index> {
    at: I,
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum Action<I: Index, T: Clone> {
    Insert(Insert<I, T>),
    Remove(Remove<I>),
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum Indication<I: Index, T: Clone> {
    Copy(I),
    Original(T),
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct Indicate<I: Index, T: Clone> {
    reduction_of: Option<I>,
    indicates: Indication<I, T>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct Extend<I: Index> {
    reduction_of: Option<I>,
    extends: I,
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum Instruction<I: Index, T: Clone> {
    Indicate(Indicate<I, T>),
    Extend(Extend<I>),
}

struct CopyInstructor<'a, I: Index, T: Clone> {
    previous: Option<I>,
    current: I,
    actions: &'a [Action<I, T>],
    done: bool,
}

impl<'a, I: Index, T: Clone> CopyInstructor<'a, I, T> {
    fn new(start: I, actions: &'a [Action<I, T>]) -> Self {
        Self {
            previous: None,
            current: start,
            actions,
            done: false,
        }
    }
}

impl<'a, I: Index, T: Clone> Iterator for CopyInstructor<'a, I, T> {
    type Item = Instruction<I, T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.actions.split_first() {
                Some((first, rest)) => {
                    let i = match first {
                        Action::Insert(Insert { at: i, object: _ }) => i,
                        Action::Remove(Remove { at: i }) => i,
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

        let instruction = match self.actions.first() {
            Some(action) => match action {
                Action::Insert(Insert { at, object }) => Some(Instruction::Indicate(Indicate {
                    reduction_of: self.previous.clone(),
                    indicates: if self.current.directly_indicates(at) {
                        Indication::Original(object.clone())
                    } else {
                        Indication::Copy(self.current.indicate())
                    },
                })),
                Action::Remove(Remove { at }) => {
                    todo!();
                }
            },
            None => {
                if self.done {
                    None
                } else {
                    self.done = true;

                    Some(Instruction::Extend(Extend {
                        reduction_of: self.previous.clone(),
                        extends: self.current.clone(),
                    }))
                }
            }
        };

        self.previous = Some(self.current.clone());
        self.current.reduce_mut();

        instruction
    }
}

struct RecursiveCopyInstructor<'a, I: Index, T: Clone> {
    queue: VecDeque<CopyInstructor<'a, I, T>>,
}

impl<'a, I: Index, T: Clone> RecursiveCopyInstructor<'a, I, T> {
    fn new(start: I, actions: &'a [Action<I, T>]) -> Self {
        let mut queue = VecDeque::with_capacity(1);
        queue.push_back(CopyInstructor::new(start, &actions[..]));

        Self { queue }
    }
}

impl<'a, I: Index, T: Clone> Iterator for RecursiveCopyInstructor<'a, I, T> {
    type Item = Instruction<I, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut instructor = self.queue.pop_front()?;

        match instructor.next() {
            Some(instruction) => {
                let instruction_copy = instruction.clone();

                match instruction {
                    Instruction::Indicate(Indicate {
                        reduction_of: _,
                        indicates: i,
                    }) => {
                        let actions = instructor.actions;
                        self.queue.push_back(instructor);

                        match i {
                            Indication::Copy(copy) => {
                                let nested_instructor = CopyInstructor::new(copy, actions);
                                self.queue.push_back(nested_instructor);

                                Some(instruction_copy)
                            }
                            Indication::Original(_) => Some(instruction_copy),
                        }
                    }
                    Instruction::Extend(_) => Some(instruction_copy),
                }
            }
            None => {
                panic!("copy instructor finished before expected");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_0() {
        let start = bitvec![];
        let actions = &[Action::Insert(Insert {
            at: bitvec![1],
            object: "a",
        })][..];
        let mut ci = CopyInstructor::new(start, actions);
        assert_eq!(
            Some(Instruction::Indicate(Indicate {
                reduction_of: None,
                indicates: Indication::Original("a"),
            })),
            ci.next()
        );
        assert_eq!(
            Some(Instruction::Extend(Extend {
                reduction_of: Some(bitvec![]),
                extends: bitvec![0],
            })),
            ci.next()
        );
        assert_eq!(None, ci.next());
    }

    #[test]
    fn ci_1() {
        let start = bitvec![];
        let actions = &[Action::Insert(Insert {
            at: bitvec![0, 1],
            object: "a",
        })][..];
        let mut ci = CopyInstructor::new(start, actions);
        assert_eq!(
            Some(Instruction::Indicate(Indicate {
                reduction_of: None,
                indicates: Indication::Copy(bitvec![1]),
            })),
            ci.next()
        );
        assert_eq!(
            Some(Instruction::Indicate(Indicate {
                reduction_of: Some(bitvec![]),
                indicates: Indication::Original("a"),
            })),
            ci.next(),
        );
        assert_eq!(
            Some(Instruction::Extend(Extend {
                reduction_of: Some(bitvec![0]),
                extends: bitvec![0, 0],
            })),
            ci.next()
        );
        assert_eq!(None, ci.next());
    }

    #[test]
    fn ci_2() {
        let start = bitvec![];
        let actions = &[Action::Insert(Insert {
            at: bitvec![0, 0, 1],
            object: "a",
        })][..];
        let mut ci = CopyInstructor::new(start, actions);
        assert_eq!(
            Some(Instruction::Indicate(Indicate {
                reduction_of: None,
                indicates: Indication::Copy(bitvec![1]),
            })),
            ci.next()
        );
        assert_eq!(
            Some(Instruction::Indicate(Indicate {
                reduction_of: Some(bitvec![]),
                indicates: Indication::Copy(bitvec![0, 1]),
            })),
            ci.next()
        );
        assert_eq!(
            Some(Instruction::Indicate(Indicate {
                reduction_of: Some(bitvec![0]),
                indicates: Indication::Original("a"),
            })),
            ci.next(),
        );
        assert_eq!(
            Some(Instruction::Extend(Extend {
                reduction_of: Some(bitvec![0, 0]),
                extends: bitvec![0, 0, 0],
            })),
            ci.next()
        );
        assert_eq!(None, ci.next());
    }

    #[test]
    fn ci_3() {
        let start = bitvec![];
        let actions = &[Action::Insert(Insert {
            at: bitvec![1, 1],
            object: "a",
        })][..];
        let mut ci = CopyInstructor::new(start, actions);
        assert_eq!(
            Some(Instruction::Indicate(Indicate {
                reduction_of: None,
                indicates: Indication::Copy(bitvec![1]),
            })),
            ci.next()
        );
        assert_eq!(
            Some(Instruction::Extend(Extend {
                reduction_of: Some(bitvec![]),
                extends: bitvec![0],
            })),
            ci.next()
        );
        assert_eq!(None, ci.next());
    }

    #[test]
    fn ci_4() {
        let start = bitvec![];
        let actions = &[Action::Insert(Insert {
            at: bitvec![1, 0, 0, 1, 0, 1],
            object: "a",
        })][..];
        let mut ci = CopyInstructor::new(start, actions);
        assert_eq!(
            Some(Instruction::Indicate(Indicate {
                reduction_of: None,
                indicates: Indication::Copy(bitvec![1]),
            })),
            ci.next()
        );
        assert_eq!(
            Some(Instruction::Extend(Extend {
                reduction_of: Some(bitvec![]),
                extends: bitvec![0],
            })),
            ci.next()
        );
        assert_eq!(None, ci.next());
    }

    #[test]
    fn ci_5() {
        let start = bitvec![0];
        let actions = &[Action::Insert(Insert {
            at: bitvec![1, 0, 0, 1, 0, 1],
            object: "a",
        })][..];
        let mut ci = CopyInstructor::new(start, actions);
        assert_eq!(
            Some(Instruction::Extend(Extend {
                reduction_of: None,
                extends: bitvec![0],
            })),
            ci.next()
        );
        assert_eq!(None, ci.next());
    }

    #[test]
    fn ci_6() {
        let start = bitvec![];
        let actions = &[
            Action::Insert(Insert {
                at: bitvec![0, 1],
                object: "a",
            }),
            Action::Insert(Insert {
                at: bitvec![0, 0, 1],
                object: "b",
            }),
        ][..];
        let mut ci = CopyInstructor::new(start, actions);
        // assert_eq!(
        //     Some(Instruction::Indicate(Indicate {
        //         reduction_of: None,
        //     }))
        //     ci.next()
        // );
        assert_eq!(None, ci.next());
    }

    // #[test]
    // fn ci_3() {
    //     let start = bitvec![];
    //     let actions = &[Action::Insert(Insert {
    //         at: bitvec![1],
    //         object: "a",
    //     })][..];
    //     let mut ci = CopyInstructor::new(start, actions);
    //     assert_eq!(
    //         Some(Instruction::Indicate(Indicate {
    //             reduction_of: None,
    //             indicates: Indication::Copy(bitvec![1]),
    //         })),
    //         ci.next()
    //     );
    //     assert_eq!(
    //         Some(Instruction::Indicate(Indicate {
    //             reduction_of: Some(bitvec![]),
    //             indicates: Indication::Copy(bitvec![0, 1]),
    //         })),
    //         ci.next()
    //     );
    //     assert_eq!(
    //         Some(Instruction::Indicate(Indicate {
    //             reduction_of: Some(bitvec![0]),
    //             indicates: Indication::Original("a"),
    //         })),
    //         ci.next(),
    //     );
    //     assert_eq!(
    //         Some(Instruction::Extend(Extend {
    //             reduction_of: bitvec![0, 0],
    //             extends: bitvec![0, 0, 0],
    //         })),
    //         ci.next()
    //     );
    //     assert_eq!(None, ci.next());
    // }
}
