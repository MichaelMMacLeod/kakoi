use petgraph::{graph::NodeIndex, stable_graph::StableGraph, Directed};

use crate::sphere::Sphere;

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

impl Tree {
    pub fn indications_of(&self, node: NodeIndex<u32>) -> Vec<(Sphere, NodeIndex<u32>)> {
        let mut walker = self
            .g
            .neighbors_directed(node, petgraph::Direction::Outgoing)
            .detach();

        let mut indications = Vec::new();

        while let Some((_, indication)) = walker.next(&self.g) {
            indications.push((
                self.g[indication].sphere,
                self.g[indication].flat_graph_index,
            ));
        }

        indications
    }
}
