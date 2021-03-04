// use crate::adapter::Adapter;
// use crate::copy_instructor::CopyInstructor;
// use crate::index::Index;
// use std::collections::VecDeque;
//
// struct RecursiveCopyInstructor<'a, I, S, A>
// where
//     I: Index,
//     S: Copy,
//     A: Adapter<S>,
// {
//     start: I,
//     queue: VecDeque<S>,
//     adapter: &'a mut A,
// }
//
// impl<'a, I, S, A> RecursiveCopyInstructor<'a, I, S, A>
// where
//     I: Index,
//     S: Copy,
//     A: Adapter<S>,
// {
//     fn new(start: I, S, adapter: &'a mut A) -> Self {
//         let mut queue = VecDeque::with_capacity(1);
//         queue.push_back(source);
//         Self { queue, adapter }
//     }
// }
//
// impl<'a, I, S, A> Iterator for RecursiveCopyInstructor<'a, I, S, A>
// where
//     I: Index,
//     S: Copy,
//     A: Adapter<S>,
// {
//     type Item = ();
//
//     fn next(&mut self) -> Option<Self::Item> {
//         let front = self.queue.pop_front()?;
//
//         let copy_instructor = CopyInstructor::new();
//     }
// }
