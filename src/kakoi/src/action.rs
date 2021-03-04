use crate::index::Index;

pub enum Action<I: Index, S> {
    Insert(I, S),
    Remove(I),
}

impl<I: Index, S> Action<I, S> {
    pub fn index(&self) -> &I {
        match self {
            Action::Insert(i, _) => i,
            Action::Remove(i) => i,
        }
    }
}
