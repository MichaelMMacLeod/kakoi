use crate::action::Action;
use crate::graph::{Edge, Graph, Node};
use crate::index::Index;
use petgraph::graph::NodeIndex;
use std::iter::Peekable;

pub struct CopyInstructor<'a, 'c, I, SI, AI>
where
    I: 'a + Index,
    SI: IntoIterator<Item = (NodeIndex<u32>, NodeIndex<u32>)>,
    AI: IntoIterator<Item = &'a Action<I, NodeIndex<u32>>>,
{
    current_source: I,
    current_copy: I,
    source: SI::IntoIter,
    actions: Peekable<AI::IntoIter>,
    graph: &'c mut Graph,
    done: bool,
    previous: Option<NodeIndex<u32>>,
}

impl<'a, 'c, I, SI, AI> CopyInstructor<'a, 'c, I, SI, AI>
where
    I: 'a + Index,
    SI: IntoIterator<Item = (NodeIndex<u32>, NodeIndex<u32>)>,
    AI: IntoIterator<Item = &'a Action<I, NodeIndex<u32>>>,
{
    pub fn new(start: I, source: SI, actions: AI, graph: &'c mut Graph) -> Self {
        Self {
            current_source: start.clone(),
            current_copy: start,
            source: source.into_iter(),
            actions: actions.into_iter().peekable(),
            done: false,
            graph,
            previous: None,
        }
    }

    fn process_immediate_direct_insertion(
        &mut self,
        object_to_insert: &NodeIndex<u32>,
    ) -> Status<I, NodeIndex<u32>> {
        self.current_source.reduce_mut();
        self.current_copy.reduce_mut();
        self.actions.next();

        let n0 = self.graph.insert();
        if let Some(p) = self.previous {
            self.graph.extend(p, n0);
        }
        self.previous = Some(n0);
        self.graph.indicate(n0, *object_to_insert);

        Status::Processed(None)
    }

    fn process_immediate_indirect_insertion(&mut self) -> Status<I, NodeIndex<u32>> {
        match self.source.next() {
            Some((_, to)) => {
                let source_i = self.current_source.indicate();

                self.current_source.reduce_mut();
                self.current_copy.reduce_mut();
                self.actions.next();

                let n0 = self.graph.insert();
                if let Some(p) = self.previous {
                    self.graph.extend(p, n0);
                }
                self.previous = Some(n0);
                let n1 = self.graph.insert();
                self.graph.indicate(n0, n1);

                Status::Processed(Some(Recurse {
                    index: source_i,
                    source: to,
                    copy: n1,
                }))
            }
            None => todo!(), // probably panic here
        }
    }

    fn process_delayed_insertion(&mut self) -> Status<I, NodeIndex<u32>> {
        match self.source.next() {
            Some((_, to)) => {
                self.current_source.reduce_mut();
                self.current_copy.reduce_mut();

                let n0 = self.graph.insert();
                if let Some(p) = self.previous {
                    self.graph.extend(p, n0);
                }
                self.previous = Some(n0);
                self.graph.indicate(n0, to);

                Status::Processed(None)
            }
            None => Status::Done,
        }
    }

    fn process_immediate_direct_removal(&mut self) -> Status<I, NodeIndex<u32>> {
        self.source.next();
        self.current_source.reduce_mut();
        self.actions.next();

        Status::NotDone
    }

    fn process_immediate_indirect_removal(&mut self) -> Status<I, NodeIndex<u32>> {
        match self.source.next() {
            Some((_, to)) => {
                let source_i = self.current_source.indicate();

                self.current_source.reduce_mut();
                self.current_copy.reduce_mut();
                self.actions.next();

                let n0 = self.graph.insert();
                if let Some(p) = self.previous {
                    self.graph.extend(p, n0);
                }
                self.previous = Some(n0);
                let n1 = self.graph.insert();
                self.graph.indicate(n0, n1);

                Status::Processed(Some(Recurse {
                    index: source_i,
                    source: to,
                    copy: n1,
                }))
            }
            None => todo!(), // probably panic here
        }
    }

    fn process_delayed_removal(&mut self) -> Status<I, NodeIndex<u32>> {
        match self.source.next() {
            Some((_, to)) => {
                self.current_source.reduce_mut();
                self.current_copy.reduce_mut();

                let n0 = self.graph.insert();
                if let Some(p) = self.previous {
                    self.graph.extend(p, n0);
                }
                self.previous = Some(n0);
                self.graph.indicate(n0, to);

                Status::Processed(None)
            }
            None => todo!(), // probably panic here
        }
    }

    fn process_extension(&mut self) -> Status<I, NodeIndex<u32>> {
        match self.source.next() {
            Some((from, _)) => {
                if let Some(p) = self.previous {
                    self.graph.extend(p, from);
                }

                Status::Done
            }
            None => Status::Done,
        }
    }

    fn process_action(&mut self) -> Status<I, NodeIndex<u32>> {
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

impl<'a, 'c, I, SI, AI> Iterator for CopyInstructor<'a, 'c, I, SI, AI>
where
    I: 'a + Index,
    SI: IntoIterator<Item = (NodeIndex<u32>, NodeIndex<u32>)>,
    AI: IntoIterator<Item = &'a Action<I, NodeIndex<u32>>>,
{
    type Item = Recurse<I, NodeIndex<u32>>;

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
pub struct Recurse<I, S>
where
    I: Index,
    S: Copy,
{
    index: I,
    source: S,
    copy: S,
}

enum Status<I, S>
where
    I: Index,
    S: Copy,
{
    Done,
    NotDone,
    Processed(Option<Recurse<I, S>>),
}

impl<I, S> Status<I, S>
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
