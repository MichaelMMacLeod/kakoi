use crate::index::Index;
use petgraph::graph::NodeIndex;

#[derive(Eq, PartialEq, Debug)]
pub struct Recurse<I>
where
    I: Index,
{
    pub index: I,
    pub source: NodeIndex<u32>,
    pub copy: NodeIndex<u32>,
}
