#![allow(unused)]

use petgraph::data::DataMap;
use petgraph::graph::Graph as GraphImpl;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Directed;
use petgraph::Direction;

#[derive(Debug)]
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
}

struct ReductionIterator<'a> {
    graph: &'a Graph,
    node: NodeIndex<u32>,
    first_reduction: bool,
}

impl<'a> ReductionIterator<'a> {
    fn new(graph: &'a Graph, node: NodeIndex<u32>) -> Self {
        Self {
            graph,
            node,
            first_reduction: true,
        }
    }
}

impl<'a> Iterator for ReductionIterator<'a> {
    type Item = NodeIndex<u32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first_reduction {
            self.first_reduction = false;
            Some(self.node)
        } else {
            if let Some(reduction) = self.graph.reduction_of(self.node) {
                self.node = reduction;
                Some(self.node)
            } else {
                None
            }
        }
    }
}

struct IndicationIterator<'a> {
    graph: &'a Graph,
    reduction_iterator: ReductionIterator<'a>,
}

impl<'a> IndicationIterator<'a> {
    fn new(graph: &'a Graph, node: NodeIndex<u32>) -> Self {
        Self {
            graph,
            reduction_iterator: ReductionIterator::new(graph, node),
        }
    }
}

impl<'a> Iterator for IndicationIterator<'a> {
    type Item = NodeIndex<u32>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(reduction) = self.reduction_iterator.next() {
                if let Some(indication) = self.graph.indication_of(reduction) {
                    break Some(indication);
                }
            } else {
                break None;
            }
        }
    }
}

struct GroupIterator<'a> {
    graph: &'a Graph,
    reduction_iterator: ReductionIterator<'a>,
}

impl<'a> GroupIterator<'a> {
    fn new(graph: &'a Graph, node: NodeIndex<u32>) -> Self {
        Self {
            graph,
            reduction_iterator: ReductionIterator::new(graph, node),
        }
    }
}

impl<'a> Iterator for GroupIterator<'a> {
    type Item = (NodeIndex<u32>, NodeIndex<u32>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(reduction) = self.reduction_iterator.next() {
                if let Some(indication) = self.graph.indication_of(reduction) {
                    break Some((reduction, indication));
                }
            } else {
                break None;
            }
        }
    }
}

impl Graph {
    fn new() -> Self {
        let mut g = GraphImpl::new();

        Self { g }
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

    fn new_group_with_leaf(&mut self, leaf: String) -> NodeIndex<u32> {
        let branch = self.g.add_node(Node::Branch);

        self.extend_with_leaf(branch, leaf)
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
        IndicationIterator::new(&self, group).collect::<Vec<_>>()
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

    fn extend_replace_indices(
        &mut self,
        m: NodeIndex<u32>,
        replacements: Vec<(usize, NodeIndex<u32>)>,
    ) -> NodeIndex<u32> {
        let mut groups = GroupIterator::new(&self, m).collect::<Vec<_>>();
        let mut last_update = None;

        for (indication, replacement) in replacements {
            dbg!(groups[indication].1, replacement);
            if groups[indication].1 != replacement {
                groups[indication].1 = replacement;

                last_update = Some(indication);
            }
        }

        match last_update {
            None => m,
            Some(last_update) => {
                let new_m = self.g.add_node(Node::Branch);
                self.g.add_edge(new_m, groups[0].1, Edge::Indication);
                let mut current = new_m;

                for (_, indication) in &groups[1..=last_update] {
                    let next = self.g.add_node(Node::Branch);
                    self.g.add_edge(next, *indication, Edge::Indication);
                    self.g.add_edge(current, next, Edge::Extension);
                    current = next;
                }

                dbg!(last_update, groups.len());
                if last_update + 1 < groups.len() {
                    self.g
                        .add_edge(current, groups[last_update + 1].0, Edge::Extension);
                }
                new_m
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
        let n = g.new_group_with_leaf("Hello, world!".to_string());
        let i = g.indication_of(n).unwrap();
        assert_eq!(g.leaf_value(i).unwrap(), "Hello, world!");
    }

    #[test]
    fn indicate_two_leafs() {
        let mut g = Graph::new();
        let n1 = g.new_group_with_leaf("A".to_string());
        let n2 = g.extend_with_leaf(n1, "B".to_string());
        assert_eq!(g.leaf_value(g.indication_of(n2).unwrap()).unwrap(), "B");
        assert_eq!(g.reduction_of(n2).unwrap(), n1);
        assert_eq!(
            g.leaf_value(g.indication_of(g.reduction_of(n2).unwrap()).unwrap())
                .unwrap(),
            "A"
        );
    }

    #[test]
    fn indications_of_three() {
        let mut g = Graph::new();
        let n1 = g.new_group_with_leaf("a".to_string());
        let n2 = g.extend_with_leaf(n1, "b".to_string());
        let n3 = g.extend_with_leaf(n2, "c".to_string());
        let v = g.indications_of(n3);
        //dbg!(g.g.node_weight(v[0]));
        //dbg!(g.g.node_weight(v[1]));
        //panic!();
        assert_eq!(v[0], g.indication_of(n3).unwrap());
        assert_eq!(v[1], g.indication_of(n2).unwrap());
        assert_eq!(v[2], g.indication_of(n1).unwrap());
    }

    #[test]
    fn group_iterator_of_three() {
        let mut g = Graph::new();
        let n1 = g.new_group_with_leaf("a".to_string());
        let n2 = g.extend_with_leaf(n1, "b".to_string());
        let n3 = g.extend_with_leaf(n2, "c".to_string());
        let mut i = GroupIterator::new(&g, n3);
        assert_eq!(i.next(), Some((n3, g.indication_of(n3).unwrap())));
        assert_eq!(i.next(), Some((n2, g.indication_of(n2).unwrap())));
        assert_eq!(i.next(), Some((n1, g.indication_of(n1).unwrap())));
        assert_eq!(i.next(), None);
    }

    #[test]
    fn group_iterator_immediate_end() {
        // let mut g = Graph::new();
        // let mut i = GroupIterator::new(&g, );
        // assert_eq!(i.next(), None);
    }

    #[test]
    fn extend_until_same() {
        let mut g = Graph::new();
        let n0 = g.new_group_with_leaf("0".to_string());
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
        let n0 = g.new_group_with_leaf("0".to_string());
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
        let n0 = g.new_group_with_leaf("0".to_string());
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
        let n0 = g.new_group_with_leaf("0".to_string());
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
        let n0 = g.new_group_with_leaf("0".to_string());
        let n1 = g.extend_with_leaf(n0, "1".to_string());
        let n2 = g.extend_with_leaf(n1, "2".to_string());
        let n3 = g.extend_with_leaf(n2, "3".to_string());
        let (v, b, r) = g.extend_until(n2, g.indication_of(n1).unwrap());
        assert_eq!(b, v);
        assert_eq!(g.leaf_value(g.indication_of(v).unwrap()).unwrap(), "2");
        assert!(g.reduction_of(v).is_none());
        assert_eq!(n1, r);
    }

    #[test]
    fn extend_replace_indices_no_changes() {
        let mut g = Graph::new();
        let n0 = g.new_group_with_leaf("a".to_string());
        let n1 = g.extend_with_leaf(n0, "b".to_string());
        let n2 = g.extend_with_leaf(n1, "c".to_string());
        let n3 = g.extend_with_leaf(n2, "d".to_string());
        let replacements = vec![];
        let result = g.extend_replace_indices(n3, replacements);
        assert_eq!(result, n3)
    }

    #[test]
    fn extend_replace_indices_change_0_to_1() {
        let mut g = Graph::new();
        let n0 = g.new_group_with_leaf("a".to_string());
        let n1 = g.extend_with_leaf(n0, "b".to_string());
        let n2 = g.extend_with_leaf(n1, "c".to_string());
        let n3 = g.extend_with_leaf(n2, "d".to_string());
        let replacements = vec![(0, g.indication_of(n2).unwrap())];
        let result = g.extend_replace_indices(n3, replacements);
        let mut i = IndicationIterator::new(&g, result);
        dbg!(g.indication_of(i.next().unwrap()));
        assert_ne!(result, n3);
        assert_eq!(
            g.indication_of(result).unwrap(),
            g.indication_of(n2).unwrap()
        );
        assert_eq!(n2, g.reduction_of(result).unwrap());
    }

    #[test]
    fn extend_replace_indices_change_two_useless_changes() {
        let mut g = Graph::new();
        let n0 = g.new_group_with_leaf("a".to_string());
        let n1 = g.extend_with_leaf(n0, "b".to_string());
        let n2 = g.extend_with_leaf(n1, "c".to_string());
        let n3 = g.extend_with_leaf(n2, "d".to_string());
        let replacements = vec![
            (1, g.indication_of(n2).unwrap()),
            (3, g.indication_of(n0).unwrap()),
        ];
        let result = g.extend_replace_indices(n3, replacements);
        assert_eq!(result, n3)
    }

    #[test]
    fn extend_replace_indices_change_1_to_3_and_3_to_1() {
        let mut g = Graph::new();
        let n0 = g.new_group_with_leaf("a".to_string());
        let n1 = g.extend_with_leaf(n0, "b".to_string());
        let n2 = g.extend_with_leaf(n1, "c".to_string());
        let n3 = g.extend_with_leaf(n2, "d".to_string());
        let replacements = vec![
            (1, g.indication_of(n0).unwrap()),
            (3, g.indication_of(n2).unwrap()),
        ];
        let result = g.extend_replace_indices(n3, replacements);
        assert_ne!(result, n3);
        assert_eq!(g.leaf_value(g.indication_of(result).unwrap()).unwrap(), "d");
        let r1 = g.reduction_of(result).unwrap();
        assert_eq!(g.leaf_value(g.indication_of(r1).unwrap()).unwrap(), "a");
        let r2 = g.reduction_of(r1).unwrap();
        assert_eq!(g.leaf_value(g.indication_of(r2).unwrap()).unwrap(), "b");
        let r3 = g.reduction_of(r2).unwrap();
        assert_eq!(g.leaf_value(g.indication_of(r3).unwrap()).unwrap(), "c");
        assert_eq!(None, g.reduction_of(r3));
    }

    #[test]
    fn extend_replace_indices_large_overlap() {
        let mut g = Graph::new();
        let n0 = g.new_group_with_leaf("a".to_string());
        let n1 = g.extend_with_leaf(n0, "b".to_string());
        let n2 = g.extend_with_leaf(n1, "c".to_string());
        let n3 = g.extend_with_leaf(n2, "d".to_string());
        let replacements = vec![(0, g.indication_of(n2).unwrap())];
        let result = g.extend_replace_indices(n3, replacements);
        assert_ne!(result, n3);
        assert_ne!(result, n2);
        assert_ne!(result, n1);
        assert_ne!(result, n0);
        assert_eq!(g.leaf_value(g.indication_of(result).unwrap()).unwrap(), "c");
        assert_eq!(g.reduction_of(result).unwrap(), n2);
    }

    #[test]
    fn extend_replace_indices_change_last() {
        let mut g = Graph::new();
        let n0 = g.new_group_with_leaf("a".to_string());
        let n1 = g.extend_with_leaf(n0, "b".to_string());
        let n2 = g.extend_with_leaf(n1, "c".to_string());
        let n3 = g.extend_with_leaf(n2, "d".to_string());
        let replacements = vec![(3, g.indication_of(n1).unwrap())];
        let result = g.extend_replace_indices(n3, replacements);
        assert_ne!(result, n3);
        assert_ne!(result, n2);
        assert_ne!(result, n1);
        assert_ne!(result, n0);
        let r0 = result;
        assert_eq!(g.leaf_value(g.indication_of(r0).unwrap()).unwrap(), "d");
        let r1 = g.reduction_of(r0).unwrap();
        assert_eq!(g.leaf_value(g.indication_of(r1).unwrap()).unwrap(), "c");
        let r2 = g.reduction_of(r1).unwrap();
        assert_eq!(g.leaf_value(g.indication_of(r2).unwrap()).unwrap(), "b");
        let r3 = g.reduction_of(r2).unwrap();
        assert_eq!(g.leaf_value(g.indication_of(r3).unwrap()).unwrap(), "b");
        assert_eq!(None, g.reduction_of(r3));
    }
}
