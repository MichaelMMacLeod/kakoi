use crate::store;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph as GraphImpl;
use petgraph::Directed;

#[derive(Debug)]
pub struct Edge;

#[derive(Debug)]
pub struct Node;

pub struct FlatGraph {
    pub g: GraphImpl<Node, Edge, Directed, u32>,
    pub focused: Option<store::Key>,
}

// struct Todo {
//     source: NodeIndex<u32>,
//     copy: NodeIndex<u32>,
// }

pub enum Group {
    Existing { key: store::Key, index: usize },
    New,
}

pub enum Insertion {
    Existing(store::Key),
    New(store::Value),
}

impl FlatGraph {
    pub fn new() -> Self {
        FlatGraph {
            g: GraphImpl::new(),
            focused: None,
        }
    }

    fn get_target(&mut self, store: &mut store::Store, insertion: Insertion) -> store::Key {
        match insertion {
            Insertion::New(value) => {
                let target = store::Key::from(self.g.add_node(Node));
                store.entry(target).or_insert(value);
                target
            }
            Insertion::Existing(target) => target,
        }
    }

    fn insert_target_at(
        &mut self,
        store: &mut store::Store,
        source: store::Key,
        target: store::Key,
        index: usize,
    ) {
        let route = self
            .g
            .add_edge(NodeIndex::from(source), NodeIndex::from(target), Edge);
        store
            .entry(source)
            .and_modify(|value| match value.association_mut() {
                Some(store::Association { indications, .. }) => {
                    indications.insert(index, store::Indication { target, route });
                }
                None => panic!("attempt to insert into non-association"),
            });
    }

    fn create_association(
        &mut self,
        store: &mut store::Store,
        insertions: &mut Vec<Insertion>,
    ) -> store::Key {
        let association = store::Key::from(self.g.add_node(Node));
        let indications = insertions
            .drain(..)
            .map(|insertion| {
                let target = self.get_target(store, insertion);
                let route =
                    self.g
                        .add_edge(NodeIndex::from(association), NodeIndex::from(target), Edge);
                store::Indication { target, route }
            })
            .collect();
        store
            .entry(association)
            .or_insert(store::Value::Association(store::Association::new(
                indications,
                0,
                store::Association::to_zoom(0.0),
            )));
        association
    }

    // Creates or locates a group that indicates `insertions`, possibly by
    // modifying an existing group. Returns its index if such a group would have
    // at least one indication, otherwise, returns None.
    //
    // The returned index can point to either a Node::Branch or a Node::Leaf,
    // depending on the arguments.
    pub fn enclose(
        &mut self,
        store: &mut store::Store,
        into: Group,
        insertions: &mut Vec<Insertion>,
    ) -> Option<store::Key> {
        match into {
            Group::New => match insertions.len() {
                0 => None,
                1 => Some(self.get_target(store, insertions.drain(..).next().unwrap())),
                _ => Some(self.create_association(store, insertions)),
            },
            Group::Existing { key: source, index } => match insertions.len() {
                0 => Some(source),
                1 => {
                    let target = self.get_target(store, insertions.drain(..).next().unwrap());
                    self.insert_target_at(store, source, target, index);
                    Some(source)
                }
                _ => {
                    let target = self.create_association(store, insertions);
                    self.insert_target_at(store, source, target, index);
                    Some(source)
                }
            },
        }
    }

    // // Flattens a source graph into a FlatGraph.
    // //
    // // Only groups indicated by the focused node in the source graph will be
    // // indicated in the FlatGraph.
    // //
    // // While the Graph type is acyclic, a FlatGraph need not be.
    // pub fn from_source(source_graph: &Graph) -> Self {
    //     let mut copy_graph = GraphImpl::<Node, Edge, Directed, u32>::new();

    //     let focused_source = source_graph.focused.unwrap(); // TODO: figure out what to do here
    //     let focused_copy = copy_graph.add_node(Node::Branch(Branch::new(0)));

    //     let mut todo_queue = VecDeque::new();
    //     todo_queue.push_back(Todo {
    //         source: focused_source,
    //         copy: focused_copy,
    //     });

    //     let mut identity_map = HashMap::new();
    //     identity_map.insert(focused_source, focused_copy);

    //     while let Some(Todo { source, copy }) = todo_queue.pop_front() {
    //         FlatGraph::from_source_helper(
    //             &mut copy_graph,
    //             source_graph,
    //             source,
    //             copy,
    //             &mut todo_queue,
    //             &mut identity_map,
    //         );
    //     }

    //     FlatGraph {
    //         g: copy_graph,
    //         focused: Some(focused_copy),
    //     }
    // }

    // // Helper function for from_source that should only be called from
    // // from_source.
    // //
    // // Processes a single enclose in the source graph. Before this function is
    // // called a copy of the start of the enclose is present in the flat graph.
    // // After this function is called, that node will have edges emanating out of
    // // it which point to copies of the nodes that the corresponding enclose in the
    // // source graph indicates. Each of the indicated nodes are then added to a
    // // queue so they can each, in turn, be processed by this function.
    // //
    // // copy_graph: the flat graph that is being constructed
    // // source_graph: the graph that is being flattened
    // // source: an index being currently flattened in the source_graph
    // // copy: the index corresponding to `source` in copy_graph.
    // // todo_queue: this function will push_back nodes that the enclose `source`
    // //             indicates so that they can be later processed by this
    // //             function
    // // identity_map: A single node in the copy graph corresponds to one or more
    // //               nodes in the source graph. When we encounter a source node
    // //               we map it to its copy here. This is necessary when the
    // //               source contains more than one indication of a node; both of
    // //               those indications must resolve to the same copy in our flat
    // //               graph.
    // fn from_source_helper(
    //     copy_graph: &mut GraphImpl<Node, Edge, Directed, u32>,
    //     source_graph: &Graph,
    //     source: NodeIndex<u32>,
    //     copy: NodeIndex<u32>,
    //     todo_queue: &mut VecDeque<Todo>,
    //     identity_map: &mut HashMap<NodeIndex<u32>, NodeIndex<u32>>,
    // ) {
    //     let mut counter = 0;
    //     let mut current = source_graph.reduce_until_indication(source);
    //     let mut needs_identity_update = false;

    //     while let Some((current_source, current_source_indication)) = current {
    //         if needs_identity_update {
    //             identity_map.insert(current_source, copy);
    //         } else {
    //             needs_identity_update = true;
    //         }

    //         let indicated_copy = match identity_map.get(&current_source_indication) {
    //             Some(indicated_copy) => *indicated_copy,
    //             None => match source_graph.g.node_weight(current_source_indication) {
    //                 Some(GraphNode::Branch) => {
    //                     let indicated_copy = copy_graph.add_node(Node::Branch(Branch::new(0)));
    //                     identity_map.insert(current_source_indication, indicated_copy);
    //                     todo_queue.push_back(Todo {
    //                         source: current_source_indication,
    //                         copy: indicated_copy,
    //                     });
    //                     indicated_copy
    //                 }
    //                 Some(GraphNode::Leaf(s)) => {
    //                     let copy = copy_graph.add_node(Node::Leaf(s.to_string()));
    //                     identity_map.insert(current_source_indication, copy);
    //                     copy
    //                 }
    //                 None => {
    //                     panic!("Node vanished from source graph.");
    //                 }
    //             },
    //         };

    //         copy_graph.add_edge(copy, indicated_copy, Edge(counter));

    //         counter += 1;

    //         source_graph.next_source(&mut current);
    //     }

    //     *&mut copy_graph[copy] = Node::Branch(Branch::new(counter));
    // }

    // pub fn double_cycle_example() -> Self {
    //     let mut graph = FlatGraph::new();
    //     let cycle_1 = Self::make_leaf_insertions(&["Cycle #1"]);
    //     let cycle_2 = Self::make_leaf_insertions(&["Cycle #2"]);
    //     let double_cycle = Self::make_leaf_insertions(&["Double cycle.\nHow intimidating!"]);
    //     let cycle_1_index = {
    //         let mut dc1 = double_cycle.clone();
    //         dc1.append(&mut cycle_1.clone());
    //         let c1idx = graph.enclose(Group::New, dc1).unwrap();
    //         graph
    //             .enclose(
    //                 Group::Existing {
    //                     index: c1idx,
    //                     position: 2,
    //                 },
    //                 vec![Insertion::Existing { index: c1idx }],
    //             )
    //             .unwrap()
    //     };
    //     let cycle_2_index = {
    //         let mut dc2 = double_cycle.clone();
    //         dc2.append(&mut cycle_2.clone());
    //         let c2idx = graph.enclose(Group::New, dc2).unwrap();
    //         graph
    //             .enclose(
    //                 Group::Existing {
    //                     index: c2idx,
    //                     position: 2,
    //                 },
    //                 vec![Insertion::Existing { index: c2idx }],
    //             )
    //             .unwrap()
    //     };
    //     let toplevel = graph
    //         .enclose(
    //             Group::New,
    //             vec![
    //                 Insertion::Existing {
    //                     index: cycle_1_index,
    //                 },
    //                 Insertion::Existing {
    //                     index: cycle_2_index,
    //                 },
    //             ],
    //         )
    //         .unwrap();
    //     graph.focused = Some(toplevel);
    //     graph
    // }

    fn make_leaf_insertions<'a>(leafs: &[&str]) -> Vec<Insertion> {
        leafs
            .iter()
            .map(|&c| Insertion::New(store::Value::String(c.into())))
            .collect()
    }

    pub fn naming_example<'a>(store: &'a mut store::Store) -> Self {
        let mut graph = FlatGraph::new();
        let mut consonants = Self::make_leaf_insertions(&[
            "b", "c", "d", "f", "g", "h", "j", "k", "l", "m", "n", "p", "q", "r", "s", "t", "v",
            "w", "x", "y", "z",
        ]);
        let consonant_index = graph.enclose(store, Group::New, &mut consonants).unwrap();
        let mut vowels = Self::make_leaf_insertions(&["a", "e", "i", "o", "u"]);
        let vowel_index = graph.enclose(store, Group::New, &mut vowels).unwrap();
        let named_consonant_index = graph
            .enclose(
                store,
                Group::New,
                &mut vec![
                    Insertion::Existing(consonant_index),
                    Insertion::New(store::Value::String("Consonant".into())),
                ],
            )
            .unwrap();
        let named_vowel_index = graph
            .enclose(
                store,
                Group::New,
                &mut vec![
                    Insertion::Existing(vowel_index),
                    Insertion::New(store::Value::String("Vowel".into())),
                ],
            )
            .unwrap();
        let named_kakoi_examples_index = {
            let kakoi_examples_index = {
                let kakoi_example_1 = {
                    let kakoi_example_1 =
                        include_bytes!("resources/images/Kakoi Example 1 [senseis.xmp.net].png");
                    image::load_from_memory(kakoi_example_1)
                        .unwrap()
                        .into_rgba8()
                };
                let kakoi_example_2 = {
                    let kakoi_example_2 =
                        include_bytes!("resources/images/Kakoi Example 2 [senseis.xmp.net].png");
                    image::load_from_memory(kakoi_example_2)
                        .unwrap()
                        .into_rgba8()
                };
                let kakoi_example_3 = {
                    let kakoi_example_3 = include_bytes!(
                        "resources/images/Kakoi Example 1 [senseis.xmp.net] wide.png"
                    );
                    image::load_from_memory(kakoi_example_3)
                        .unwrap()
                        .into_rgba8()
                };
                graph
                    .enclose(
                        store,
                        Group::New,
                        &mut vec![
                            Insertion::New(store::Value::Image(kakoi_example_1)),
                            Insertion::New(store::Value::Image(kakoi_example_2)),
                            Insertion::New(store::Value::Image(kakoi_example_3)),
                        ],
                    )
                    .unwrap()
            };
            graph
                .enclose(
                    store,
                    Group::New,
                    &mut vec![
                        Insertion::Existing(kakoi_examples_index),
                        Insertion::New(store::Value::String("kakoi".into())),
                    ],
                )
                .unwrap()
        };
        let name_index = graph
            .enclose(
                store,
                Group::New,
                &mut vec![
                    Insertion::Existing(named_consonant_index),
                    Insertion::Existing(named_vowel_index),
                    Insertion::Existing(named_kakoi_examples_index),
                ],
            )
            .unwrap();
        let named_name_index = graph
            .enclose(
                store,
                Group::New,
                &mut vec![
                    Insertion::Existing(name_index),
                    Insertion::New(store::Value::String("Naming".into())),
                ],
            )
            .unwrap();
        graph.enclose(
            store,
            Group::Existing {
                key: name_index,
                index: 0,
            },
            &mut vec![Insertion::Existing(named_name_index)],
        );
        graph.focused = Some(name_index);
        graph
    }
}
