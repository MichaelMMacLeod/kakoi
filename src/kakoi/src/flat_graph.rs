use crate::graph::{Graph, Node as GraphNode};
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph as GraphImpl;
use petgraph::Directed;
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub struct Edge(pub u32);

#[derive(Debug)]
pub enum Node {
    Branch(u32),
    Leaf(String),
}

impl Node {
    pub fn branch_value(&self) -> u32 {
        match self {
            Node::Branch(value) => *value,
            _ => panic!("not a branch"),
        }
    }
}

pub struct FlatGraph {
    pub g: GraphImpl<Node, Edge, Directed, u32>,
    pub focused: Option<NodeIndex<u32>>,
}

struct Todo {
    source: NodeIndex<u32>,
    copy: NodeIndex<u32>,
}

pub enum Group {
    Existing {
        index: NodeIndex<u32>,
        position: u32,
    },
    New,
}

pub enum Insertion {
    Existing { index: NodeIndex<u32> },
    New { leaf: String },
}

impl FlatGraph {
    pub fn new() -> Self {
        FlatGraph {
            g: GraphImpl::new(),
            focused: None,
        }
    }

    // Prepares a group for the addition of an indication at a specified
    // position.
    //
    // `enclose` must point to a Node::Branch. This is checked, and will panic
    // if not satisfied.
    fn prepare_group(&mut self, enclose: NodeIndex<u32>, position: u32) {
        // TODO: check for overflow bugs here

        let branch = &mut self.g[enclose];
        match branch {
            Node::Branch(n) => *branch = Node::Branch(*n + 1),
            Node::Leaf(_) => panic!("attempted to insert into leaf"),
        }

        use petgraph::visit::EdgeRef;
        let edges = self
            .g
            .edges(enclose)
            .into_iter()
            .map(|r| r.id())
            .collect::<Vec<_>>();
        for edge in edges {
            let edge = &mut self.g[edge];
            match edge {
                Edge(n) if *n >= position => {
                    *edge = Edge(*n + 1);
                }
                _ => {}
            }
        }
    }

    // Adds indications of `members`, in order, into the group specified by
    // `enclose`.
    //
    // `enclose` must point to a Node::Branch. This is not checked.
    //
    // `members` should be of length at least two. This is not checked.
    fn populate_empty_group(&mut self, enclose: NodeIndex<u32>, members: Vec<Insertion>) {
        let mut current_position = 0;

        for member in members {
            match member {
                Insertion::Existing { index: indication } => {
                    self.g.add_edge(enclose, indication, Edge(current_position));
                }
                Insertion::New { leaf } => {
                    let indication = self.g.add_node(Node::Leaf(leaf));
                    self.g.add_edge(enclose, indication, Edge(current_position));
                }
            }
            current_position += 1;
        }
    }

    // Creates or locates a group that indicates `members`, possibly by
    // modifying an existing group. Returns its index if such a group would have
    // at least one indication, otherwise, returns None.
    //
    // The returned index can point to either a Node::Branch or a Node::Leaf,
    // depending on the arguments.
    pub fn enclose(&mut self, into: Group, members: Vec<Insertion>) -> Option<NodeIndex<u32>> {
        use std::convert::TryInto;

        let num_indications = members.len().try_into().unwrap();

        match into {
            Group::Existing { index, position } => match num_indications {
                0 => Some(index),
                1 => {
                    let node_to_insert = match &members[0] {
                        Insertion::New { leaf } => self.g.add_node(Node::Leaf(leaf.clone())),
                        Insertion::Existing { index } => *index,
                    };
                    self.prepare_group(index, position);
                    self.g.add_edge(index, node_to_insert, Edge(position));
                    Some(index)
                }
                _ => {
                    let node_to_insert = self.g.add_node(Node::Branch(num_indications));
                    self.prepare_group(index, position);
                    self.g.add_edge(index, node_to_insert, Edge(position));
                    self.populate_empty_group(node_to_insert, members);
                    Some(index)
                }
            },
            Group::New => match num_indications {
                0 => None,
                1 => match &members[0] {
                    Insertion::New { leaf } => Some(self.g.add_node(Node::Leaf(leaf.clone()))),
                    Insertion::Existing { index } => Some(*index),
                },
                _ => {
                    let node_to_insert = self.g.add_node(Node::Branch(num_indications));
                    self.populate_empty_group(node_to_insert, members);
                    Some(node_to_insert)
                }
            },
        }
    }

    // Flattens a source graph into a FlatGraph.
    //
    // Only groups indicated by the focused node in the source graph will be
    // indicated in the FlatGraph.
    //
    // While the Graph type is acyclic, a FlatGraph need not be.
    pub fn from_source(source_graph: &Graph) -> Self {
        let mut copy_graph = GraphImpl::<Node, Edge, Directed, u32>::new();

        let focused_source = source_graph.focused.unwrap(); // TODO: figure out what to do here
        let focused_copy = copy_graph.add_node(Node::Branch(0));

        let mut todo_queue = VecDeque::new();
        todo_queue.push_back(Todo {
            source: focused_source,
            copy: focused_copy,
        });

        let mut identity_map = HashMap::new();
        identity_map.insert(focused_source, focused_copy);

        while let Some(Todo { source, copy }) = todo_queue.pop_front() {
            FlatGraph::from_source_helper(
                &mut copy_graph,
                source_graph,
                source,
                copy,
                &mut todo_queue,
                &mut identity_map,
            );
        }

        FlatGraph {
            g: copy_graph,
            focused: Some(focused_copy),
        }
    }

    // Helper function for from_source that should only be called from
    // from_source.
    //
    // Processes a single enclose in the source graph. Before this function is
    // called a copy of the start of the enclose is present in the flat graph.
    // After this function is called, that node will have edges emanating out of
    // it which point to copies of the nodes that the corresponding enclose in the
    // source graph indicates. Each of the indicated nodes are then added to a
    // queue so they can each, in turn, be processed by this function.
    //
    // copy_graph: the flat graph that is being constructed
    // source_graph: the graph that is being flattened
    // source: an index being currently flattened in the source_graph
    // copy: the index corresponding to `source` in copy_graph.
    // todo_queue: this function will push_back nodes that the enclose `source`
    //             indicates so that they can be later processed by this
    //             function
    // identity_map: A single node in the copy graph corresponds to one or more
    //               nodes in the source graph. When we encounter a source node
    //               we map it to its copy here. This is necessary when the
    //               source contains more than one indication of a node; both of
    //               those indications must resolve to the same copy in our flat
    //               graph.
    fn from_source_helper(
        copy_graph: &mut GraphImpl<Node, Edge, Directed, u32>,
        source_graph: &Graph,
        source: NodeIndex<u32>,
        copy: NodeIndex<u32>,
        todo_queue: &mut VecDeque<Todo>,
        identity_map: &mut HashMap<NodeIndex<u32>, NodeIndex<u32>>,
    ) {
        let mut counter = 0;
        let mut current = source_graph.reduce_until_indication(source);
        let mut needs_identity_update = false;

        while let Some((current_source, current_source_indication)) = current {
            if needs_identity_update {
                identity_map.insert(current_source, copy);
            } else {
                needs_identity_update = true;
            }

            let indicated_copy = match identity_map.get(&current_source_indication) {
                Some(indicated_copy) => *indicated_copy,
                None => match source_graph.g.node_weight(current_source_indication) {
                    Some(GraphNode::Branch) => {
                        let indicated_copy = copy_graph.add_node(Node::Branch(0));
                        identity_map.insert(current_source_indication, indicated_copy);
                        todo_queue.push_back(Todo {
                            source: current_source_indication,
                            copy: indicated_copy,
                        });
                        indicated_copy
                    }
                    Some(GraphNode::Leaf(s)) => {
                        let copy = copy_graph.add_node(Node::Leaf(s.to_string()));
                        identity_map.insert(current_source_indication, copy);
                        copy
                    }
                    None => {
                        panic!("Node vanished from source graph.");
                    }
                },
            };

            copy_graph.add_edge(copy, indicated_copy, Edge(counter));

            counter += 1;

            source_graph.next_source(&mut current);
        }

        *&mut copy_graph[copy] = Node::Branch(counter);
    }

    pub fn naming_example() -> Self {
        fn make_leaf_insertions(leafs: &[&str]) -> Vec<Insertion> {
            leafs
                .iter()
                .map(|&c| Insertion::New { leaf: c.into() })
                .collect()
        }
        let mut graph = FlatGraph::new();
        let consonants = make_leaf_insertions(&[
            "b", "c", "d", "f", "g", "h", "j", "k", "l", "m", "n", "p", "q", "r", "s", "t", "v",
            "w", "x", "y", "z",
        ]);
        let consonant_index = graph.enclose(Group::New, consonants).unwrap();
        let vowels = make_leaf_insertions(&["a", "e", "i", "o", "u"]);
        let vowel_index = graph.enclose(Group::New, vowels).unwrap();
        let named_consonant_index = graph
            .enclose(
                Group::New,
                vec![
                    Insertion::Existing {
                        index: consonant_index,
                    },
                    Insertion::New {
                        leaf: "Consonant".into(),
                    },
                ],
            )
            .unwrap();
        let named_vowel_index = graph
            .enclose(
                Group::New,
                vec![
                    Insertion::Existing { index: vowel_index },
                    Insertion::New {
                        leaf: "Vowel".into(),
                    },
                ],
            )
            .unwrap();
        let name_index = graph
            .enclose(
                Group::New,
                vec![
                    Insertion::Existing {
                        index: named_consonant_index,
                    },
                    Insertion::Existing {
                        index: named_vowel_index,
                    },
                ],
            )
            .unwrap();
        let named_name_index = graph
            .enclose(
                Group::New,
                vec![
                    Insertion::Existing { index: name_index },
                    Insertion::New {
                        leaf: "Naming".into(),
                    },
                ],
            )
            .unwrap();
        graph.enclose(
            Group::Existing {
                index: name_index,
                position: 0,
            },
            vec![Insertion::Existing {
                index: named_name_index,
            }],
        );
        graph.focused = Some(name_index);
        graph
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use petgraph::dot::Dot;

    #[test]
    fn example_1() {
        let graph = FlatGraph::naming_example();
        eprintln!("{:?}", Dot::with_config(&graph.g, &[]));
    }

    #[test]
    fn naming_example_0() {
        //
        // let graph = FlatGraph::from_source(&mut Graph::make_double_example());

        // eprintln!("{:?}", Dot::with_config(&graph.g, &[]));

        // panic!("PRINT THE GRAPH PLEASE"); // uncomment me to see the the graph in graphviz dot
        // You can use a program like xdot to view it.
    }
}
