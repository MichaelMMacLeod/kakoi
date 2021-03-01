#![allow(unused)]

use crate::graph::{Edge, Graph, GraphImpl, Node};
use crate::index::Index;
use crate::index::IndicationClass;
use petgraph::data::DataMap;
use petgraph::graph::IndexType;
use petgraph::graph::NodeIndex;
use petgraph::visit::Bfs;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::cmp::Ordering;
use std::collections::VecDeque;

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

    fn sort_replacements(replacements: &mut Vec<(Vec<u32>, Option<NodeIndex<u32>>)>) -> () {
        replacements.sort_unstable_by(|a, b| {
            let mut a_iter = a.0.iter();
            let mut b_iter = b.0.iter();

            let result = loop {
                let ai = a_iter.next();
                let bi = b_iter.next();

                if ai.is_none() || bi.is_none() {
                    panic!("Un-sortable replacements"); // TODO: verify
                } else {
                    let a = ai.unwrap();
                    let b = bi.unwrap();

                    match a.cmp(b) {
                        Ordering::Equal => continue,
                        Ordering::Less => break Some(Ordering::Less),
                        Ordering::Greater => break Some(Ordering::Greater),
                    }
                }
            };

            result.unwrap_or(Ordering::Equal)
        });
    }

    // enum {
    //     IndicateCopy,
    //     IndicteOriginal,
    // }

    // // extend_queue: (index, original_node, copy_node)[]
    // fn process_indication(
    //     &mut self,
    //     node: Index<u32>,
    //     replacements: IntoIterator<Item = Index<u32>>,
    // ) {
    // }

    fn extend_replace_nested_indices(
        &mut self,
        top: NodeIndex<u32>,
        top_index: Index<u32>,
        replacements: &mut Vec<(Index<u32>, NodeIndex<u32>)>,
    ) -> NodeIndex<u32> {
        todo!();
        // let tightest_classification = |node: &Index<u32>| -> Option<IndicationClass> {};

        // // let indicates_any =
        // //     |node: &Index<u32>| -> bool { replacements.iter().any(|r| r.0.indicates(node)) };

        // if replacements.is_empty() {
        //     top
        // } else {
        //     let mut extend_queue = VecDeque::new();

        //     extend_queue.push_back((top_index, top, None));

        //     let mut return_value = None;

        //     while !extend_queue.is_empty() {
        //         let (index, node, previous) = extend_queue.pop_front().unwrap();

        //         // let previous = previous_option.unwrap_or_else(|| self.g.add_node(Node::Branch));

        //         if let Some(indication) = self.indication_of(node) {
        //             let c_node = self.g.add_node(Node::Branch);

        //             match self.g.node_weight(indication).unwrap() {
        //                 Node::Leaf(_) => {
        //                     self.g.add_edge(c_node, indication, Edge::Indication);
        //                 }
        //                 Node::Branch => {
        //                     let c_index = index.indicate(0);

        //                     match tightest_classification(&c_index) {
        //                         Some(IndicationClass::Direct) => {
        //                             self.g.add_edge(c_node, indication, Edge::Indication);
        //                         }
        //                         Some(IndicationClass::Indirect) => {
        //                             let c_indication = self.g.add_node(Node::Branch);

        //                             self.g.add_edge(c_node, c_indication, Edge::Indication);

        //                             extend_queue.push_back((
        //                                 c_index.clone(),
        //                                 indication,
        //                                 Some(c_indication),
        //                             ));
        //                         }
        //                         None => {
        //                             self.g.add_edge(c_node, indication, Edge::Indication);
        //                         }
        //                     }
        //                 }
        //             }

        //             if let Some(p) = previous {
        //                 self.g.add_edge(p, c_node, Edge::Extension);
        //             } else {
        //                 if let None = return_value {
        //                     return_value = Some(c_node);
        //                 }
        //             }

        //             if let Some(r) = self.reduction_of(node) {
        //                 let r_index = index.reduce();

        //                 match tightest_classification(&r_index) {
        //                     Some(IndicationClass::Direct) | Some(IndicationClass::Indirect) => {
        //                         extend_queue.push_back((r_index.clone(), r, Some(c_node)));
        //                     }
        //                     None => {
        //                         self.g.add_edge(c_node, r, Edge::Extension);
        //                     }
        //                 }
        //             }
        //         }
        //     }

        //     return_value.unwrap()
        // }
    }

    // fn extend_replace_nested_indices(
    //     &mut self,
    //     context: NodeIndex<u32>,
    //     replacements: &mut Vec<(Vec<u32>, Option<NodeIndex<u32>>)>,
    // ) -> NodeIndex<u32> {
    //     if replacements.is_empty() {
    //         context
    //     } else {
    //         Graph::sort_replacements(replacements);

    //         let mut repl_it = replacements.iter();

    //         let top = self.g.add_node(Node::Branch);
    //         let mut last_node = top;

    //         loop {
    //             let mut current = repl_it.next();
    //         }
    //     }

    //     context
    // }

    // we need something like
    // CircleBFSExplorer(startNode: NodeIndex<u32>)
    //   - &mut self, followIndication: bool, exploreRestOfCircle: bool -> { index: &Vec<u32>, node: NodeIndex<u32> }
    //     - traverses the circle in a pseudo bfs/dfs manner:
    //       - fully explores exactly one group (or until exploreRestOfCircle provided is false)
    //       - when done exploring the circle, explore the circles indicated when followIndication was true.
    //       - ... and so on.
    //     - the index provided is relative to the start node.
    //     - the start node is the first one returned, so its index is vec![0].
    //     - idk how recursive groups should be handled. Probably just treated as non-recursive.

    // fn explore_from_to_with<T: IndexType, F>(
    //     &self,
    //     start: &mut Index<T>,
    //     from: NodeIndex<T>,
    //     to: &Index<T>,
    //     f: F,
    // ) where
    //     F: Fn(Index<T>, NodeIndex<T>),
    // {
    // }
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

    #[test]
    fn sort_replacements_1() {
        let mut replacements = vec![(vec![0, 1, 2], None), (vec![1, 2, 0], None)];
        let expected = vec![(vec![0, 1, 2], None), (vec![1, 2, 0], None)];
        Graph::sort_replacements(&mut replacements);
        assert_eq!(expected, replacements);
    }

    #[test]
    fn sort_replacements_2() {
        let mut replacements = vec![(vec![0, 1, 2], None), (vec![0, 2, 0], None)];
        let expected = vec![(vec![0, 1, 2], None), (vec![0, 2, 0], None)];
        Graph::sort_replacements(&mut replacements);
        assert_eq!(expected, replacements);
    }

    #[test]
    fn sort_replacements_3() {
        let mut replacements = vec![(vec![0, 1, 2], None), (vec![0, 0, 0], None)];
        let expected = vec![(vec![0, 0, 0], None), (vec![0, 1, 2], None)];
        Graph::sort_replacements(&mut replacements);
        assert_eq!(expected, replacements);
    }

    // fn build_replace_nested_indices_test_graph_0() -> Graph {
    // }

    #[test]
    fn extend_replace_nested_indices_0() {
        let mut graph = Graph::new();
        let n1 = graph.new_group_with_leaf("n1".to_string());
        let n2 = graph.extend_with_leaf(n1, "n2".to_string());
        let n3 = graph.extend_with_leaf(n2, "n3".to_string());
        let n4 = graph.extend_with_leaf(n3, "n4".to_string());

        let m1 = graph.new_group_with_leaf("m1".to_string());
        let m1_leaf = graph.indication_of(m1).unwrap();

        let result = graph.extend_replace_nested_indices(
            n4,
            Index::from(vec![0]),
            &mut vec![(Index::from(vec![2]), m1_leaf)],
        );

        let vs = GroupIterator::new(&graph, result).collect::<Vec<_>>();

        assert_eq!(vs.len(), 4);

        let leaf_of = |n| graph.indication_of(n).unwrap();

        assert_eq!(vs[0].1, leaf_of(n4));
        assert_eq!(vs[1].1, leaf_of(n3));
        assert_eq!(vs[2].1, leaf_of(m1));
        assert_eq!(vs[3].1, leaf_of(n1));
    }
}
