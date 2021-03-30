use petgraph::{graph::NodeIndex, Directed, Graph};

use crate::flat_graph::FlatGraphIndex;

pub struct TreeNode {
    pub indication_tree_index: TreeIndex,
    pub flat_graph_index: FlatGraphIndex,
}

pub type TreeEdge = ();

pub type Impl = Graph<TreeNode, TreeEdge, Directed, u32>;

pub struct TreeIndex {
    index: NodeIndex<u32>,
}

impl From<TreeIndex> for NodeIndex<u32> {
    fn from(tree_index: TreeIndex) -> Self {
        tree_index.index
    }
}

impl From<NodeIndex<u32>> for TreeIndex {
    fn from(node_index: NodeIndex<u32>) -> Self {
        Self { index: node_index }
    }
}

pub struct Tree {
    pub g: Impl,
    pub root: TreeIndex,
}