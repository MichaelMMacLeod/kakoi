// pub trait Adapter<'i, I, S, G>
// where
//     I: 'i,
//     S: Copy,
//     G: GroupIterator<'i, I, S>,
// {
//     fn insert(&mut self) -> S;
//     fn extend(&mut self, from: S, to: S);
//     fn indicate(&mut self, from: S, to: S);
//     fn iterate_from(&self, from: S) -> G;
// }
//
// pub trait GroupIterator<'i, I: 'i, S: Copy>: Iterator<Item = (S, S)> {}
// // impl<'i, I: 'i, S: Copy, T: Iterator<Item = (S, S)>> GroupIterator<'i, I, S> for T {}
