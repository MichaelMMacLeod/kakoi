use crate::action::Action;
use crate::adapter::Adapter;
use crate::graph::{Edge, Graph, Node};
use crate::index::Index;
use std::iter::Peekable;

pub struct CopyInstructor<'a, 'b, 'c, I, S, SI, AI, A>
where
    I: 'a + Index,
    S: 'a + 'b + Copy,
    SI: IntoIterator<Item = &'b (S, S)>,
    AI: IntoIterator<Item = &'a Action<I, S>>,
    A: Adapter<S>,
{
    current_source: I,
    current_copy: I,
    source: SI::IntoIter,
    actions: Peekable<AI::IntoIter>,
    adapter: &'c mut A,
    done: bool,
    previous: Option<S>,
}

impl<'a, 'b, 'c, I, S, SI, AI, A> CopyInstructor<'a, 'b, 'c, I, S, SI, AI, A>
where
    I: 'a + Index,
    S: 'a + 'b + Copy,
    SI: IntoIterator<Item = &'b (S, S)>,
    AI: IntoIterator<Item = &'a Action<I, S>>,
    A: Adapter<S>,
{
    pub fn new(start: I, source: SI, actions: AI, adapter: &'c mut A) -> Self {
        Self {
            current_source: start.clone(),
            current_copy: start,
            source: source.into_iter(),
            actions: actions.into_iter().peekable(),
            done: false,
            adapter,
            previous: None,
        }
    }

    fn process_immediate_direct_insertion(&mut self, object_to_insert: &S) -> Status<S> {
        self.current_source.reduce_mut();
        self.current_copy.reduce_mut();
        self.actions.next();

        let n0 = self.adapter.insert();
        if let Some(p) = self.previous {
            self.adapter.extend(p, n0);
        }
        self.previous = Some(n0);
        self.adapter.indicate(n0, *object_to_insert);

        Status::Processed(None)
    }

    fn process_immediate_indirect_insertion(&mut self) -> Status<S> {
        match self.source.next() {
            Some((_, to)) => {
                self.current_source.reduce_mut();
                self.current_copy.reduce_mut();
                self.actions.next();

                let n0 = self.adapter.insert();
                if let Some(p) = self.previous {
                    self.adapter.extend(p, n0);
                }
                self.previous = Some(n0);
                let n1 = self.adapter.insert();
                self.adapter.indicate(n0, n1);

                Status::Processed(Some(Recurse {
                    source: *to,
                    copy: n1,
                }))
            }
            None => todo!(), // probably panic here
        }
    }

    fn process_delayed_insertion(&mut self) -> Status<S> {
        match self.source.next() {
            Some((_, to)) => {
                self.current_source.reduce_mut();
                self.current_copy.reduce_mut();

                let n0 = self.adapter.insert();
                if let Some(p) = self.previous {
                    self.adapter.extend(p, n0);
                }
                self.previous = Some(n0);
                self.adapter.indicate(n0, *to);

                Status::Processed(None)
            }
            None => Status::Done,
        }
    }

    fn process_immediate_direct_removal(&mut self) -> Status<S> {
        self.source.next();
        self.current_source.reduce_mut();
        self.actions.next();

        Status::NotDone
    }

    fn process_immediate_indirect_removal(&mut self) -> Status<S> {
        match self.source.next() {
            Some((_, to)) => {
                self.current_source.reduce_mut();
                self.current_copy.reduce_mut();
                self.actions.next();

                let n0 = self.adapter.insert();
                if let Some(p) = self.previous {
                    self.adapter.extend(p, n0);
                }
                self.previous = Some(n0);
                let n1 = self.adapter.insert();
                self.adapter.indicate(n0, n1);

                Status::Processed(Some(Recurse {
                    source: *to,
                    copy: n1,
                }))
            }
            None => todo!(), // probably panic here
        }
    }

    fn process_delayed_removal(&mut self) -> Status<S> {
        match self.source.next() {
            Some((_, to)) => {
                self.current_source.reduce_mut();
                self.current_copy.reduce_mut();

                let n0 = self.adapter.insert();
                if let Some(p) = self.previous {
                    self.adapter.extend(p, n0);
                }
                self.previous = Some(n0);
                self.adapter.indicate(n0, *to);

                Status::Processed(None)
            }
            None => todo!(), // probably panic here
        }
    }

    fn process_extension(&mut self) -> Status<S> {
        match self.source.next() {
            Some((from, _)) => {
                if let Some(p) = self.previous {
                    self.adapter.extend(p, *from);
                }

                Status::Done
            }
            None => Status::Done,
        }
    }

    fn process_action(&mut self) -> Status<S> {
        let action = loop {
            let action = self.actions.peek();

            match action {
                Some(a) => {
                    if !self.current_copy.indicates(a.index()) {
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
                        self.process_immediate_direct_insertion(object)
                    } else if self.current_copy.indirectly_indicates(index) {
                        self.process_immediate_indirect_insertion()
                    } else {
                        self.process_delayed_insertion()
                    }
                }
                Action::Remove(index) => {
                    if self.current_source.directly_indicates(index) {
                        self.process_immediate_direct_removal()
                    } else if self.current_source.indirectly_indicates(index) {
                        self.process_immediate_indirect_removal()
                    } else {
                        self.process_delayed_removal()
                    }
                }
            },
            None => {
                if self.done {
                    Status::Done
                } else {
                    self.done = true;

                    self.process_extension()
                }
            }
        };

        instruction
    }
}

impl<'a, 'b, 'c, I, S, SI, AI, A> Iterator for CopyInstructor<'a, 'b, 'c, I, S, SI, AI, A>
where
    I: 'a + Index,
    S: 'a + 'b + Copy,
    SI: IntoIterator<Item = &'b (S, S)>,
    AI: IntoIterator<Item = &'a Action<I, S>>,
    A: Adapter<S>,
{
    type Item = Recurse<S>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut instruction = Status::NotDone;

            while instruction.is_not_done() {
                instruction = self.process_action();
            }

            match instruction {
                Status::NotDone => panic!("unreachable state reached"),
                Status::Done => break None,
                Status::Processed(r) => {
                    if r.is_some() {
                        break r;
                    }
                }
            }
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct Recurse<S>
where
    S: Copy,
{
    source: S,
    copy: S,
}

enum Status<S>
where
    S: Copy,
{
    Done,
    NotDone,
    Processed(Option<Recurse<S>>),
}

impl<S> Status<S>
where
    S: Copy,
{
    fn is_not_done(&self) -> bool {
        match self {
            Status::NotDone => true,
            _ => false,
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use bitvec::prelude::*;

    #[derive(Eq, PartialEq, Debug)]
    enum TesterAction {
        Insert { id: u32 },
        Extend { from: u32, to: u32 },
        Indicate { from: u32, to: u32 },
    }

    struct CopyInstructorTester {
        current: u32,
        actions: Vec<TesterAction>,
    }

    impl CopyInstructorTester {
        fn new() -> Self {
            Self {
                current: 0,
                actions: Vec::new(),
            }
        }
    }

    impl Adapter<u32> for CopyInstructorTester {
        fn insert(&mut self) -> u32 {
            let v = self.current;
            self.current += 1;
            self.actions.push(TesterAction::Insert { id: v });
            v
        }

        fn extend(&mut self, from: u32, to: u32) {
            self.actions.push(TesterAction::Extend { from, to });
        }

        fn indicate(&mut self, from: u32, to: u32) {
            self.actions.push(TesterAction::Indicate { from, to });
        }
    }

    #[test]
    fn copy_instructor_0() {
        let start = bitvec![];
        let source = &[(105, 104), (103, 102), (101, 100)];
        let actions = &[Action::Insert(bitvec![0, 1], 106)];
        let mut tester = CopyInstructorTester::new();
        let mut ci = CopyInstructor::new(start, source, actions, &mut tester);
        // assert_eq!(
        //     Some(Recurse {
        //         source: 101,
        //         copy: 1
        //     }),
        //     ci.next()
        // );
        assert!(ci.next().is_none());

        let mut ti = tester.actions.iter();
        assert_eq!(Some(&TesterAction::Insert { id: 0 }), ti.next());
        assert_eq!(
            Some(&TesterAction::Indicate { from: 0, to: 104 }),
            ti.next()
        );
        assert_eq!(Some(&TesterAction::Insert { id: 1 }), ti.next());
        assert_eq!(Some(&TesterAction::Extend { from: 0, to: 1 }), ti.next());
        assert_eq!(
            Some(&TesterAction::Indicate { from: 1, to: 106 }),
            ti.next()
        );
        assert_eq!(Some(&TesterAction::Extend { from: 1, to: 103 }), ti.next());
        assert!(ti.next().is_none());

        dbg!(tester.actions);
    }
}
