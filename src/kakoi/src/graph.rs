#![allow(unused)]

use petgraph::graph::Graph as GraphImpl;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Directed;
use petgraph::Direction;

enum Node {
    Leaf(String), // We're only going to support string leafs for the time being
    Branch,
}

enum Edge {
    Indication,  // in diagrams, these are the red arrows
    Extension,   // blue arrows
    Transaction, // green arrows
}

struct Graph {
    g: GraphImpl<Node, Edge, Directed, u32>,
    void: NodeIndex<u32>, // the first node; all distinctions are extensions of it
}

impl Graph {
    fn new() -> Self {
        let mut g = GraphImpl::new();

        let void = g.add_node(Node::Branch);

        Self { g, void }
    }

    fn void(&self) -> NodeIndex<u32> {
        self.void
    }

    fn extend(&mut self, to_extend: NodeIndex<u32>, to_indicate: NodeIndex<u32>) -> NodeIndex<u32> {
        let branch = self.g.add_node(Node::Branch);

        self.g.add_edge(branch, to_extend, Edge::Extension);
        self.g.add_edge(branch, to_indicate, Edge::Indication);

        branch
    }

    fn extend_with_leaf(&mut self, to_extend: NodeIndex<u32>, leaf: String) -> NodeIndex<u32> {
        let leaf_node = self.g.add_node(Node::Leaf(leaf));

        self.extend(to_extend, leaf_node)
    }

    fn indication_of(&self, group: NodeIndex<u32>) -> Option<NodeIndex<u32>> {
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

    fn reduction_of(&self, group: NodeIndex<u32>) -> Option<NodeIndex<u32>> {
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

    fn leaf_value(&self, node: NodeIndex<u32>) -> Option<&String> {
        self.g
            .node_weight(node)
            .and_then(|n| if let Node::Leaf(l) = n { Some(l) } else { None })
    }

    fn indications_of(&self, group: NodeIndex<u32>) -> Vec<NodeIndex<u32>> {
        let mut result = Vec::new();
        let mut current_group = group;

        loop {
            if let Some(i) = self.indication_of(current_group) {
                result.push(i);
            }

            if let Some(r) = self.reduction_of(current_group) {
                current_group = r;
            } else {
                break result;
            }
        }
    }

    fn extend_until(
        &mut self,
        m: NodeIndex<u32>,
        n: NodeIndex<u32>,
    ) -> (NodeIndex<u32>, NodeIndex<u32>, NodeIndex<u32>) {
        let v = self.g.add_node(Node::Branch);
        let mut b = v;
        let mut b_has_indication = false;
        let mut r = m;
        loop {
            if let Some(i) = self.indication_of(r) {
                if i == n {
                    break (v, b, r);
                } else {
                    if b_has_indication {
                        let b2 = self.g.add_node(Node::Branch);
                        self.g.add_edge(b, b2, Edge::Extension);
                        b = b2;
                    }
                    self.g.add_edge(b, i, Edge::Indication);
                    b_has_indication = true;
                }
            }

            if let Some(r2) = self.reduction_of(r) {
                r = r2;
            } else {
                panic!("n is not indicated from m");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indicate_one_leaf() {
        let mut g = Graph::new();
        let n = g.extend_with_leaf(g.void(), "Hello, world!".to_string());
        let i = g.indication_of(n).unwrap();
        assert_eq!(g.reduction_of(n).unwrap(), g.void());
        assert_eq!(g.leaf_value(i).unwrap(), "Hello, world!");
    }

    #[test]
    fn indicate_two_leafs() {
        let mut g = Graph::new();
        let n1 = g.extend_with_leaf(g.void(), "A".to_string());
        let n2 = g.extend_with_leaf(n1, "B".to_string());
        assert_eq!(g.leaf_value(g.indication_of(n2).unwrap()).unwrap(), "B");
        assert_eq!(g.reduction_of(n2).unwrap(), n1);
        assert_eq!(g.reduction_of(n1).unwrap(), g.void());
        assert_eq!(
            g.leaf_value(g.indication_of(g.reduction_of(n2).unwrap()).unwrap())
                .unwrap(),
            "A"
        );
    }

    #[test]
    fn indications_of_three() {
        let mut g = Graph::new();
        let n1 = g.extend_with_leaf(g.void(), "a".to_string());
        let n2 = g.extend_with_leaf(n1, "b".to_string());
        let n3 = g.extend_with_leaf(n2, "c".to_string());
        let v = g.indications_of(n3);
        assert_eq!(v[0], g.indication_of(n3).unwrap());
        assert_eq!(v[1], g.indication_of(n2).unwrap());
        assert_eq!(v[2], g.indication_of(n1).unwrap());
    }

    #[test]
    fn extend_until_same() {
        let mut g = Graph::new();
        let n0 = g.extend_with_leaf(g.void(), "0".to_string());
        let n1 = g.extend_with_leaf(n0, "1".to_string());
        let n2 = g.extend_with_leaf(n1, "2".to_string());
        let n3 = g.extend_with_leaf(n2, "3".to_string());
        let (v, b, r) = g.extend_until(n3, g.indication_of(n3).unwrap());
        assert!(g.indication_of(v).is_none());
        assert!(g.reduction_of(v).is_none());
        assert_eq!(b, v);
        assert_eq!(n3, r);
    }

    #[test]
    fn extend_until_1dif() {
        let mut g = Graph::new();
        let n0 = g.extend_with_leaf(g.void(), "0".to_string());
        let n1 = g.extend_with_leaf(n0, "1".to_string());
        let n2 = g.extend_with_leaf(n1, "2".to_string());
        let n3 = g.extend_with_leaf(n2, "3".to_string());
        let (v, b, r) = g.extend_until(n3, g.indication_of(n2).unwrap());
        assert_eq!(g.leaf_value(g.indication_of(v).unwrap()).unwrap(), "3");
        assert!(g.reduction_of(v).is_none());
        assert_eq!(b, v);
        assert_eq!(n2, r);
    }

    #[test]
    fn extend_until_3dif() {
        let mut g = Graph::new();
        let n0 = g.extend_with_leaf(g.void(), "0".to_string());
        let n1 = g.extend_with_leaf(n0, "1".to_string());
        let n2 = g.extend_with_leaf(n1, "2".to_string());
        let n3 = g.extend_with_leaf(n2, "3".to_string());
        let (v, b, r) = g.extend_until(n3, g.indication_of(n0).unwrap());
        assert_eq!(g.leaf_value(g.indication_of(v).unwrap()).unwrap(), "3");
        assert_eq!(
            g.leaf_value(g.indication_of(g.reduction_of(v).unwrap()).unwrap())
                .unwrap(),
            "2"
        );
        assert_eq!(
            g.leaf_value(
                g.indication_of(g.reduction_of(g.reduction_of(v).unwrap()).unwrap())
                    .unwrap()
            )
            .unwrap(),
            "1"
        );
        assert_eq!(b, g.reduction_of(g.reduction_of(v).unwrap()).unwrap());
        assert_eq!(n0, r);
    }

    #[test]
    fn extend_until_same_has_parent() {
        let mut g = Graph::new();
        let n0 = g.extend_with_leaf(g.void(), "0".to_string());
        let n1 = g.extend_with_leaf(n0, "1".to_string());
        let n2 = g.extend_with_leaf(n1, "2".to_string());
        let n3 = g.extend_with_leaf(n2, "3".to_string());
        let (v, b, r) = g.extend_until(n2, g.indication_of(n2).unwrap());
        assert!(g.indication_of(v).is_none());
        assert!(g.reduction_of(v).is_none());
        assert_eq!(b, v);
        assert_eq!(n2, r);
    }

    #[test]
    fn extend_until_1dif_has_parent() {
        let mut g = Graph::new();
        let n0 = g.extend_with_leaf(g.void(), "0".to_string());
        let n1 = g.extend_with_leaf(n0, "1".to_string());
        let n2 = g.extend_with_leaf(n1, "2".to_string());
        let n3 = g.extend_with_leaf(n2, "3".to_string());
        let (v, b, r) = g.extend_until(n2, g.indication_of(n1).unwrap());
        assert_eq!(b, v);
        assert_eq!(g.leaf_value(g.indication_of(v).unwrap()).unwrap(), "2");
        assert!(g.reduction_of(v).is_none());
        assert_eq!(n1, r);
    }
}
