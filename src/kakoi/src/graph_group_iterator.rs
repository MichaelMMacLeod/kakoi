use crate::graph::Graph;
use petgraph::graph::NodeIndex;

struct ReductionIterator<'a> {
    graph: &'a Graph,
    node: Option<NodeIndex<u32>>,
}

impl<'a> ReductionIterator<'a> {
    fn new(graph: &'a Graph, node: NodeIndex<u32>) -> Self {
        Self {
            graph,
            node: Some(node),
        }
    }
}

impl<'a> Iterator for ReductionIterator<'a> {
    type Item = NodeIndex<u32>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.node;

        if let Some(n) = self.node {
            self.node = self.graph.reduction_of(n);
        }

        node
    }
}

pub struct GraphGroupIterator<'a> {
    graph: &'a Graph,
    reduction_iterator: ReductionIterator<'a>,
}

impl<'a> GraphGroupIterator<'a> {
    pub fn new(graph: &'a Graph, node: NodeIndex<u32>) -> Self {
        Self {
            graph,
            reduction_iterator: ReductionIterator::new(graph, node),
        }
    }
}

impl<'a> Iterator for GraphGroupIterator<'a> {
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
