use crate::index::Index;

pub enum Action<I: Index, S> {
    Insert(I, S),
    Remove(I),
}
