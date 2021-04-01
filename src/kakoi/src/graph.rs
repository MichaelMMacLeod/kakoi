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

    pub fn reduce_mut(
        &self,
        node: &mut NodeIndex<u32>,
    ) -> Option<(NodeIndex<u32>, NodeIndex<u32>)> {
        let result = self.reduce(*node);
        if let Some((from, _)) = result {
            *node = from;
        }
        result
    }
}
