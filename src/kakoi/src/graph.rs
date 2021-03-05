use crate::action::Action;
use crate::copy_instructor::CopyInstructor;
use crate::graph_group_iterator::GraphGroupIterator;
use crate::index::Index;
use bitvec::prelude::*;
pub use petgraph::graph::Graph as GraphImpl;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Directed;
use petgraph::Direction;

#[derive(Debug)]
pub enum Node {
    Leaf(String), // We're only going to support string leafs for the time being
    Branch,
}

pub enum Edge {
    Indication,  // in diagrams, these are the red arrows
    Extension,   // blue arrows
    Transaction, // green arrows
}

pub struct Graph {
    pub g: GraphImpl<Node, Edge, Directed, u32>,
    pub focused: Option<NodeIndex<u32>>,
}

// pub trait Consistent {
//     fn insert(&mut self) -> NodeIndex<u32>;
//     fn extend(&mut self, from: NodeIndex<u32>, to: NodeIndex<u32>) -> NodeIndex<u32>;
//     fn indicate(&mut self, from: NodeIndex<u32>, to: NodeIndex<u32>) -> NodeIndex<u32>;
//     fn
// }

impl Graph {
    pub fn insert(&mut self) -> NodeIndex<u32> {
        self.g.add_node(Node::Branch)
    }

    pub fn extend(&mut self, from: NodeIndex<u32>, to: NodeIndex<u32>) {
        self.g.add_edge(from, to, Edge::Extension);
    }

    pub fn indicate(&mut self, from: NodeIndex<u32>, to: NodeIndex<u32>) {
        self.g.add_edge(from, to, Edge::Indication);
    }

    pub fn iterate_from(&self, from: NodeIndex<u32>) -> GraphGroupIterator {
        GraphGroupIterator::new(&self, from)
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

    pub fn process_actions<'a, AI>(&mut self, actions: AI)
    where
        AI: IntoIterator<Item = &'a Action<BitVec, NodeIndex<u32>>>,
    {
        if let Some(start) = self.focused {
            // TODO: sort indices in increasing order
            let actions = actions.into_iter().collect::<Vec<_>>();

            // TODO: to avoid reference issues, I've collected this into a
            // vector here. This really shouldn't be necessary.
            let si = self.iterate_from(start).collect::<Vec<_>>();

            let mut copy_instructor = CopyInstructor::new(bitvec![], si, actions, self);

            while let Some(_) = copy_instructor.next() {}
        }
    }
}
