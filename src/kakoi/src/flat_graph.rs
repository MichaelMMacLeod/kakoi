use crate::graph::{Graph, Node as GraphNode};
use petgraph::graph::Graph as GraphImpl;
use petgraph::graph::NodeIndex;
use petgraph::Directed;
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
struct Edge(u32);

#[derive(Debug)]
enum Node {
    Branch(u32),
    Leaf(String),
}

impl Node {
    fn branch() -> Self {
        Node::Branch(0)
    }

    fn leaf(s: String) -> Self {
        Node::Leaf(s)
    }
}

struct FlatGraph {
    g: GraphImpl<Node, Edge, Directed, u32>,
}

struct Todo {
    source: NodeIndex<u32>,
    copy: NodeIndex<u32>,
}

impl FlatGraph {
    // Flattens a source graph into a FlatGraph.
    //
    // Only groups indicated by the focused node in the source graph will be
    // indicated in the FlatGraph.
    //
    // While the Graph type is acyclic, a FlatGraph need not be.
    fn from_source(source_graph: &Graph) -> Self {
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

        FlatGraph { g: copy_graph }
    }

    // Helper function for from_source that should only be called from
    // from_source.
    //
    // Processes a single group in the source graph. Before this function is
    // called a copy of the start of the group is present in the flat graph.
    // After this function is called, that node will have edges emanating out of
    // it which point to copies of the nodes that the corresponding group in the
    // source graph indicates. Each of the indicated nodes are then added to a
    // queue so they can each, in turn, be processed by this function.
    //
    // copy_graph: the flat graph that is being constructed
    // source_graph: the graph that is being flattened
    // source: an index being currently flattened in the source_graph
    // copy: the index corresponding to `source` in copy_graph.
    // todo_queue: this function will push_back nodes that the group `source`
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
}

#[cfg(test)]
mod test {
    use super::*;
    use petgraph::dot::Dot;

    #[test]
    fn naming_example_0() {
        let graph = FlatGraph::from_source(&mut Graph::make_naming_example());

        println!("{:?}", Dot::with_config(&graph.g, &[]));

        // panic!("PRINT THE GRAPH PLEASE"); uncomment me to see the the graph in graphviz dot
        // You can use a program like xdot to view it.
    }
}
