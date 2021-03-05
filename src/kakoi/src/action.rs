use crate::index::Index;
use petgraph::graph::NodeIndex;

pub enum Action<I: Index> {
    Insert(I, NodeIndex<u32>),
    Remove(I),
}

impl<I: Index> Action<I> {
    pub fn index(&self) -> &I {
        match self {
            Action::Insert(i, _) => i,
            Action::Remove(i) => i,
        }
    }
}
