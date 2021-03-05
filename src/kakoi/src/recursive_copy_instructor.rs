use crate::action::Action;
use crate::copy_instructor::{CopyInstructor, Recurse};
use crate::graph::Graph;
use crate::index::Index;
use petgraph::graph::NodeIndex;
use std::collections::VecDeque;

struct RecursiveCopyInstructor<'a, I, AI>
where
    I: 'a + Index,
    AI: IntoIterator<Item = &'a Action<I, NodeIndex<u32>>>,
{
    queue: VecDeque<(I, NodeIndex<u32>, NodeIndex<u32>)>,
    graph: &'a mut Graph,
    actions: AI::IntoIter,
}

impl<'a, I, AI> RecursiveCopyInstructor<'a, I, AI>
where
    I: Index,
    AI: IntoIterator<Item = &'a Action<I, NodeIndex<u32>>>,
{
    fn new(start: I, actions: AI, graph: &'a mut Graph) -> Self {
        let mut queue = VecDeque::with_capacity(1);
        queue.push_back((start, graph.focused));
        Self {
            queue,
            graph,
            actions,
        }
    }

    fn process_actions(&mut self) {
        while !self.queue.is_empty() {
            let front = self.queue.pop_front()?;

            let copy_instructor = CopyInstructor::new();

            while let Some(r) = copy_instructor.next() {
                self.queue.push_back(r);
            }
        }
    }
}

// impl<'a, I, S, A> Iterator for RecursiveCopyInstructor<'a, I, S, A>
// where
//     I: Index,
//     S: Copy,
// {
//     type Item = ();
//
//     fn next(&mut self) -> Option<Self::Item> {
//         let front = self.queue.pop_front()?;
//
//         let copy_instructor = CopyInstructor::new();
//
//         while let Some(Recurse(r)) = copy_instructor.next() {
//             self.queue.push_back(r);
//         }
//     }
// }
