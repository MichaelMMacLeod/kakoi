use petgraph::graph::NodeIndex;

pub enum InsertObject {
    NewLeaf(String),
    ExistingNode(NodeIndex<u32>),
}

pub enum FlatAction {
    Insert {
        index: NodeIndex<u32>,
        object: InsertObject,
    },
    Remove {
        index: NodeIndex<u32>,
    },
}

impl FlatAction {
    pub fn index(&self) -> NodeIndex<u32> {
        match self {
            FlatAction::Insert { index, .. } => *index,
            FlatAction::Remove { index } => *index,
        }
    }
}
