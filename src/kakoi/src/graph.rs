use crate::action::Action;
use crate::index::Index;
use crate::recurse::Recurse;
use bitvec::prelude::*;
pub use petgraph::graph::Graph as GraphImpl;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Directed;
use petgraph::Direction;
use std::collections::VecDeque;
use std::iter::Peekable;

#[derive(Debug)]
pub enum Node {
    Leaf(String), // We're only going to support string leafs for the time being
    Branch,
}

#[derive(Debug)]
pub enum Edge {
    Indication,  // in diagrams, these are the red arrows
    Extension,   // blue arrows
    Transaction, // green arrows
}

pub struct Graph {
    pub g: GraphImpl<Node, Edge, Directed, u32>,
    pub focused: Option<NodeIndex<u32>>,
}

struct CIState<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>> {
    current_source: I,
    current_copy: I,
    source: NodeIndex<u32>,
    actions: Peekable<AI::IntoIter>,
    previous: Option<NodeIndex<u32>>,
    queue: VecDeque<Recurse<I>>,
}

impl<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>> CIState<'a, I, AI> {
    fn new(start: I, source: NodeIndex<u32>, actions: AI, queue: VecDeque<Recurse<I>>) -> Self {
        CIState {
            current_source: start.clone(),
            current_copy: start,
            source,
            actions: actions.into_iter().peekable(),
            previous: None,
            queue,
        }
    }
}

impl Graph {
    fn new() -> Self {
        let g = GraphImpl::new();
        Self { g, focused: None }
    }

    pub fn insert(&mut self) -> NodeIndex<u32> {
        self.g.add_node(Node::Branch)
    }

    pub fn insert_leaf(&mut self, leaf: String) -> NodeIndex<u32> {
        self.g.add_node(Node::Leaf(leaf))
    }

    pub fn extend(&mut self, from: NodeIndex<u32>, to: NodeIndex<u32>) {
        self.g.add_edge(from, to, Edge::Extension);
    }

    pub fn indicate(&mut self, from: NodeIndex<u32>, to: NodeIndex<u32>) {
        self.g.add_edge(from, to, Edge::Indication);
    }

    pub fn indication_of(&self, group: NodeIndex<u32>) -> Option<NodeIndex<u32>> {
        self.g
            .edges_directed(group, Direction::Outgoing)
            .find_map(|e| {
                if let Edge::Indication = e.weight() {
                    Some(e.target())
                } else {
                    None
                }
            })
    }

    pub fn reduction_of(&self, group: NodeIndex<u32>) -> Option<NodeIndex<u32>> {
        self.g
            .edges_directed(group, Direction::Outgoing)
            .find_map(|e| {
                if let Edge::Extension = e.weight() {
                    Some(e.target())
                } else {
                    None
                }
            })
    }

    fn reduce(&self, node: NodeIndex<u32>) -> Option<(NodeIndex<u32>, NodeIndex<u32>)> {
        let mut node = node;

        loop {
            if let Some(reduction) = self.reduction_of(node) {
                if let Some(indication) = self.indication_of(reduction) {
                    break Some((reduction, indication));
                }

                node = reduction;
            } else {
                break None;
            }
        }
    }

    fn reduce_mut(&self, node: &mut NodeIndex<u32>) -> Option<(NodeIndex<u32>, NodeIndex<u32>)> {
        let result = self.reduce(*node);
        if let Some((from, _)) = result {
            *node = from;
        }
        result
    }

    fn process_immediate_direct_insertion<
        'a,
        I: 'a + Index,
        AI: IntoIterator<Item = &'a Action<I>>,
    >(
        &mut self,
        state: &mut CIState<'a, I, AI>,
        object_to_insert: NodeIndex<u32>,
    ) {
        state.current_source.reduce_mut();
        state.current_copy.reduce_mut();
        state.actions.next();

        let n0 = self.insert();
        if let Some(p) = state.previous {
            self.extend(p, n0);
        }
        state.previous = Some(n0);
        self.indicate(n0, object_to_insert);
    }

    fn process_immediate_direct_removal<
        'a,
        I: 'a + Index,
        AI: IntoIterator<Item = &'a Action<I>>,
    >(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) {
        self.reduce_mut(&mut state.source);
        state.current_source.reduce_mut();
        state.actions.next();
    }

    fn process_immediate_indirect_action<
        'a,
        I: 'a + Index,
        AI: IntoIterator<Item = &'a Action<I>>,
    >(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) {
        // TODO: I think .unwrap() is safe here. Is it really?
        let (_, to) = self.reduce_mut(&mut state.source).unwrap();

        let source_i = state.current_source.indicate();
        state.current_copy.reduce_mut();
        state.actions.next();

        let n0 = self.insert();
        if let Some(p) = state.previous {
            self.extend(p, n0);
        }
        state.previous = Some(n0);
        let n1 = self.insert();
        self.indicate(n0, n1);

        state.queue.push_back(Recurse {
            index: source_i,
            source: to,
            copy: n1,
        });
    }

    fn process_delayed_action<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>>(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) {
        if let Some((_, to)) = self.reduce_mut(&mut state.source) {
            state.current_source.reduce_mut();
            state.current_copy.reduce_mut();

            let n0 = self.insert();
            if let Some(p) = state.previous {
                self.extend(p, n0);
            }
            state.previous = Some(n0);
            self.indicate(n0, to);
        } // TODO: maybe we need to differentiate between insert / remove here
    }

    fn process_extension<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>>(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) {
        if let Some((from, _)) = self.reduce_mut(&mut state.source) {
            if let Some(p) = state.previous {
                self.extend(p, from);
            }
        }
    }

    fn process_action<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>>(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) -> bool {
        let action = loop {
            let action = state.actions.peek();

            match action {
                Some(a) => {
                    if !state.current_copy.indicates(a.index()) {
                        state.actions.next();
                    } else {
                        break Some(a);
                    }
                }
                None => break None,
            }
        };

        match action {
            Some(action) => match action {
                Action::Insert(index, object) => {
                    if state.current_copy.directly_indicates(index) {
                        self.process_immediate_direct_insertion(state, *object);
                    } else if state.current_copy.indirectly_indicates(index) {
                        self.process_immediate_indirect_action(state);
                    } else {
                        self.process_delayed_action(state);
                    }

                    true
                }
                Action::Remove(index) => {
                    if state.current_source.directly_indicates(index) {
                        self.process_immediate_direct_removal(state);
                    } else if state.current_source.indirectly_indicates(index) {
                        self.process_immediate_indirect_action(state);
                    } else {
                        self.process_delayed_action(state);
                    }

                    true
                }
            },
            None => {
                self.process_extension(state);

                false
            }
        }
    }

    fn process_actions<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>>(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) -> NodeIndex<u32> {
        while self.process_action(state) {}
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitvec::prelude::*;
    use petgraph::dot::{Config, Dot};

    #[test]
    fn process_actions_0() {
        let mut graph = Graph::new();
        let l1 = graph.insert_leaf("leaf 1".to_string());
        let l2 = graph.insert_leaf("leaf 2".to_string());
        let l3 = graph.insert_leaf("leaf 3".to_string());
        let n0 = graph.insert();
        let actions = [
            Action::Insert(bitvec![1], l1),
            Action::Insert(bitvec![0, 1], l2),
            Action::Insert(bitvec![0, 0, 1], l3),
        ];
        let mut queue = VecDeque::new();
        let mut state = CIState::new(bitvec![], n0, &actions, queue);
        graph.process_actions(&mut state);
        println!("{:?}", Dot::with_config(&graph.g, &[]));

        // let actions2 = [Action::Remove(bitvec![0, 1])];
        // let mut queue2 = VecDeque::new();
        // let mut state2 = CIState::new(bitvec![], n0, &actions2, queue2);
        // graph.process_actions(&mut state2);
        // println!("{:?}", Dot::with_config(&graph.g, &[]));

        panic!("woo hoo");
    }
}
