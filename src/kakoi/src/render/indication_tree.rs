use petgraph::{graph::NodeIndex, Directed, Graph};

use crate::flat_graph;

pub struct TreeNode {
    pub indication_tree_index: Index,
    pub flat_graph_index: flat_graph::Index,
}

pub type TreeEdge = ();

pub type Impl = Graph<TreeNode, TreeEdge, Directed, u32>;

pub struct Index {
    index: NodeIndex<u32>,
}

impl From<Index> for NodeIndex<u32> {
    fn from(tree_index: Index) -> Self {
        tree_index.index
    }
}

impl From<NodeIndex<u32>> for Index {
    fn from(node_index: NodeIndex<u32>) -> Self {
        Self { index: node_index }
    }
}

pub struct Tree {
    pub g: Impl,
    pub root: Index,
}