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
    source: Option<(NodeIndex<u32>, NodeIndex<u32>)>,
    actions: Peekable<AI::IntoIter>,
    previous: Option<NodeIndex<u32>>,
    queue: VecDeque<Recurse<I>>,
}

impl<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>> CIState<'a, I, AI> {
    fn new(
        start: I,
        source: Option<(NodeIndex<u32>, NodeIndex<u32>)>,
        actions: AI,
        queue: VecDeque<Recurse<I>>,
    ) -> Self {
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

enum Status {
    Done,
    Processed(Option<NodeIndex<u32>>),
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

    pub fn commit(&mut self, from: NodeIndex<u32>, to: NodeIndex<u32>) {
        self.g.add_edge(from, to, Edge::Transaction);
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

    pub fn reduce_until_indication(
        &self,
        node: NodeIndex<u32>,
    ) -> Option<(NodeIndex<u32>, NodeIndex<u32>)> {
        if let Some(indication) = self.indication_of(node) {
            Some((node, indication))
        } else {
            self.reduce(node)
        }
    }

    pub fn next_source(
        &self,
        source: &mut Option<(NodeIndex<u32>, NodeIndex<u32>)>,
    ) -> Option<(NodeIndex<u32>, NodeIndex<u32>)> {
        let result = (*source)?;
        *source = self.reduce(result.0);
        Some(result)
    }

    pub fn reduce_mut(&self, node: &mut NodeIndex<u32>) -> Option<(NodeIndex<u32>, NodeIndex<u32>)> {
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
    ) -> NodeIndex<u32> {
        state.current_source.reduce_mut();
        state.current_copy.reduce_mut();
        state.actions.next();

        let n0 = self.insert();
        if let Some(p) = state.previous {
            self.extend(p, n0);
        }
        state.previous = Some(n0);
        self.indicate(n0, object_to_insert);

        n0
    }

    fn process_immediate_direct_removal<
        'a,
        I: 'a + Index,
        AI: IntoIterator<Item = &'a Action<I>>,
    >(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) {
        self.next_source(&mut state.source);
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
    ) -> NodeIndex<u32> {
        // TODO: I think .unwrap() is safe here. Is it really?
        let (_, to) = self.next_source(&mut state.source).unwrap();

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

        n0
    }

    fn process_delayed_action<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>>(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) -> Option<NodeIndex<u32>> {
        if let Some((_, to)) = self.next_source(&mut state.source) {
            state.current_source.reduce_mut();
            state.current_copy.reduce_mut();

            let n0 = self.insert();
            if let Some(p) = state.previous {
                self.extend(p, n0);
            }
            state.previous = Some(n0);
            self.indicate(n0, to);

            Some(n0)
        } else {
            // TODO: maybe we need to differentiate between insert / remove here
            None
        }
    }

    fn process_extension<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>>(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) {
        if let Some((from, _)) = self.next_source(&mut state.source) {
            if let Some(p) = state.previous {
                self.extend(p, from);
            }
        }
    }

    fn process_action<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>>(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) -> Status {
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
                        Status::Processed(Some(
                            self.process_immediate_direct_insertion(state, *object),
                        ))
                    } else if state.current_copy.indirectly_indicates(index) {
                        Status::Processed(Some(self.process_immediate_indirect_action(state)))
                    } else {
                        Status::Processed(self.process_delayed_action(state))
                    }
                }
                Action::Remove(index) => {
                    if state.current_source.directly_indicates(index) {
                        self.process_immediate_direct_removal(state);
                        Status::Processed(None)
                    } else if state.current_source.indirectly_indicates(index) {
                        Status::Processed(Some(self.process_immediate_indirect_action(state)))
                    } else {
                        Status::Processed(self.process_delayed_action(state))
                    }
                }
            },
            None => {
                self.process_extension(state);
                Status::Done
            }
        }
    }

    fn process_actions<'a, I: 'a + Index, AI: IntoIterator<Item = &'a Action<I>>>(
        &mut self,
        state: &mut CIState<'a, I, AI>,
    ) -> Option<NodeIndex<u32>> {
        // Find the first node we insert
        let result = loop {
            match self.process_action(state) {
                Status::Processed(Some(p)) => {
                    break Some(p);
                }

                // We don't always return a node. For instance, when we process
                // an action to remove a node.
                Status::Processed(None) => {}

                // If there were no applicable actions, there is nothing to do
                Status::Done => break None,
            }
        };

        if !result.is_none() {
            // Process the rest of the actions. The body of this loop is
            // intentionally empty.
            while let Status::Processed(_) = self.process_action(state) {}
        }

        result
    }

    pub fn make_naming_example() -> Self {
        let mut graph = Graph::new();
        let la = graph.insert_leaf("a".to_string());
        let lb = graph.insert_leaf("b".to_string());
        let lc = graph.insert_leaf("c".to_string());
        let ld = graph.insert_leaf("d".to_string());
        let le = graph.insert_leaf("e".to_string());
        let lf = graph.insert_leaf("f".to_string());
        let lg = graph.insert_leaf("g".to_string());
        let li = graph.insert_leaf("i".to_string());
        let lo = graph.insert_leaf("o".to_string());
        let lu = graph.insert_leaf("u".to_string());
        let lvowel = graph.insert_leaf("vowel".to_string());
        let lconsonant = graph.insert_leaf("consonant".to_string());
        let lnaming = graph.insert_leaf("naming".to_string());

        let actions1 = [
            Action::Insert(bitvec![1], la),
            Action::Insert(bitvec![0, 1], le),
            Action::Insert(bitvec![0, 0, 1], li),
            Action::Insert(bitvec![0, 0, 0, 1], lo),
            Action::Insert(bitvec![0, 0, 0, 0, 1], lu),
        ];
        let queue1 = VecDeque::new();
        let mut state1 = CIState::new(bitvec![], None, &actions1, queue1);
        let r1 = graph.process_actions(&mut state1).unwrap();

        let actions2 = [
            Action::Insert(bitvec![1], lb),
            Action::Insert(bitvec![0, 1], lc),
            Action::Insert(bitvec![0, 0, 1], ld),
            Action::Insert(bitvec![0, 0, 0, 1], lf),
            Action::Insert(bitvec![0, 0, 0, 0, 1], lg),
        ];
        let queue2 = VecDeque::new();
        let mut state2 = CIState::new(bitvec![], None, &actions2, queue2);
        let r2 = graph.process_actions(&mut state2).unwrap();

        let actions3 = [
            Action::Insert(bitvec![1], r1),
            Action::Insert(bitvec![0, 1], lvowel),
        ];
        let queue3 = VecDeque::new();
        let mut state3 = CIState::new(bitvec![], None, &actions3, queue3);
        let r3 = graph.process_actions(&mut state3).unwrap();

        let actions4 = [
            Action::Insert(bitvec![1], r2),
            Action::Insert(bitvec![0, 1], lconsonant),
        ];
        let queue4 = VecDeque::new();
        let mut state4 = CIState::new(bitvec![], None, &actions4, queue4);
        let r4 = graph.process_actions(&mut state4).unwrap();

        let actions5 = [
            Action::Insert(bitvec![1], r3),
            Action::Insert(bitvec![0, 1], r4),
        ];
        let queue5 = VecDeque::new();
        let mut state5 = CIState::new(bitvec![], None, &actions5, queue5);
        let r5 = graph.process_actions(&mut state5).unwrap();

        let actions6 = [
            Action::Insert(bitvec![1], r5),
            Action::Insert(bitvec![0, 1], lnaming),
        ];
        let queue6 = VecDeque::new();
        let mut state6 = CIState::new(bitvec![], None, &actions6, queue6);
        let r6 = graph.process_actions(&mut state6).unwrap();

        let actions7 = [Action::Insert(bitvec![1], r6)];
        let queue7 = VecDeque::new();
        let mut state7 = CIState::new(
            bitvec![],
            graph.reduce_until_indication(r5),
            &actions7,
            queue7,
        );
        let r7 = graph.process_actions(&mut state7).unwrap();

        graph.focused = Some(r7);

        // graph.commit(r2, r1);

        // println!("{:?}", Dot::with_config(&graph.g, &[]));
        // panic!("");

        graph
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use petgraph::dot::Dot;

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
        let queue = VecDeque::new();
        let mut state = CIState::new(
            bitvec![],
            graph.reduce_until_indication(n0),
            &actions,
            queue,
        );
        let r1 = graph.process_actions(&mut state).unwrap();
        graph.commit(r1, n0);
        println!("{:?}", Dot::with_config(&graph.g, &[]));

        let actions2 = [Action::Remove(bitvec![0, 1])];
        let queue2 = VecDeque::new();
        let mut state2 = CIState::new(
            bitvec![],
            graph.reduce_until_indication(r1),
            &actions2,
            queue2,
        );
        let r2 = graph.process_actions(&mut state2).unwrap();
        graph.commit(r2, r1);
        println!("{:?}", Dot::with_config(&graph.g, &[]));
    }

    // #[test]
}
