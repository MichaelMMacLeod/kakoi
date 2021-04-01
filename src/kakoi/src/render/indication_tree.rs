use petgraph::{graph::NodeIndex, Directed, stable_graph::StableGraph};

use crate::{flat_graph, sphere::Sphere};

#[derive(Debug)]
pub struct TreeNode {
    pub sphere: Sphere,
    pub flat_graph_index: NodeIndex<u32>,
}

pub type TreeEdge = ();

pub type Impl = StableGraph<TreeNode, TreeEdge, Directed, u32>;

pub struct Tree {
    pub g: Impl,
    pub root: NodeIndex<u32>,
}
