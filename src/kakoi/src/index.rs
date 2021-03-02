use petgraph::graph::IndexType;
use std::cmp::Ordering;
use std::convert::From;
use std::ops::{Add, AddAssign};
use Ordering::{Equal, Greater, Less};

pub trait IDX: IndexType + AddAssign<Self> + Add<Output = Self> {}
impl<T: IndexType + AddAssign<Self> + Add<Output = Self>> IDX for T {}

#[derive(Eq, Debug, Clone)]
pub struct Index<T: IDX>(Vec<T>);

impl<T: IDX> From<Vec<T>> for Index<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T: IDX> Ord for Index<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .iter()
            .zip(other.0.iter())
            .find_map(|(s, o)| match s.cmp(o) {
                Equal => None,
                Greater => Some(Greater),
                Less => Some(Less),
            })
            .unwrap_or(Equal)
            .then_with(|| self.0.len().cmp(&other.0.len()))
    }
}

impl<T: IDX> PartialOrd for Index<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: IDX> PartialEq for Index<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Equal
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum CopyInstruction {
    Indicate,
    Follow,
    Replace,
    Extend,
}

#[derive(Debug)]
struct CopyInstructor<'a, T: IDX> {
    index: Index<T>,
    replacements: &'a Vec<Index<T>>,
    current_replacement: usize,
    done: bool,
}

impl<'a, T: IDX> CopyInstructor<'a, T> {
    fn new(index: Index<T>, replacements: &'a Vec<Index<T>>, current_replacement: usize) -> Self {
        Self {
            index: index,
            replacements,
            current_replacement,
            done: false,
        }
    }
}

impl<'a, T: IDX> Iterator for CopyInstructor<'a, T> {
    type Item = (Index<T>, CopyInstruction);

    fn next(&mut self) -> Option<Self::Item> {
        use CopyInstruction::*;
        use IndicationClass::*;

        // if self.current_replacement == 1 {
        //     dbg!(&self.index);
        //     dbg!(self.replacements.get(self.current_replacement));
        //     dbg!(self
        //         .index
        //         .classify(self.replacements.get(self.current_replacement).unwrap()));
        //     panic!();
        // }

        match self.replacements.get(self.current_replacement) {
            Some(replacement) => {
                dbg!(replacement);

                let instruction = match self.index.classify(replacement) {
                    Some(Direct) => {
                        self.current_replacement += 1;
                        (self.index.clone(), Replace)
                    }
                    Some(Indirect) => {
                        self.current_replacement += 1;
                        (self.index.clone(), Follow)
                    }
                    Some(Reduction) => (self.index.clone(), Indicate),
                    None => {
                        self.done = true;
                        (self.index.clone(), Extend)
                    }
                };

                self.index.reduce_mut();

                Some(instruction)
            }
            None => {
                if self.done {
                    None
                } else {
                    self.done = true;
                    Some((self.index.clone(), Extend))
                }
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum IndicationClass {
    Direct,    // node indicates it directly
    Indirect,  // node indicates a group that indicates it directly or indirectly
    Reduction, // a reduction of the node indicates it directly or indirectly
}

impl Index<u32> {}

impl<T: IDX> Index<T> {
    pub fn indicates(&self, other: &Self) -> bool {
        self.0.len() <= other.0.len() && self.0.iter().zip(other.0.iter()).all(|(s, o)| s <= o)
    }

    pub fn indicate_mut(&mut self, index: T) {
        self.0.push(index);
    }

    pub fn indicate(&self, index: T) -> Self {
        let mut c = self.clone();
        c.indicate_mut(index);
        c
    }

    pub fn reduce(&self) -> Self {
        self.reduce_to(self.0[self.0.len() - 1] + T::new(1))
    }

    pub fn reduce_mut(&mut self) {
        let v_len = self.0.len();
        self.0[v_len - 1] += T::new(1);
    }

    pub fn reduce_to(&self, reduction: T) -> Self {
        let mut v = self.0[..].iter().map(|v| *v).collect::<Vec<_>>();
        let v_len = v.len();
        v[v_len - 1] = reduction;
        Self(v)
        // Self(self.0[0..self.0.len() - 1].iter().map(|v| *v).collect())
    }

    pub fn classify(&self, other: &Self) -> Option<IndicationClass> {
        //if self.0.len() == other.0.len() {
        //    if self
        //        .0
        //        .iter()
        //        .zip(other.0.iter())
        //        .take(self.0.len() - 1)
        //        .all(|(s, o)| s == 0)
        //    {
        //        if self.0.last() == other.0.last() {
        //            Some(IndicationClass::)
        //        }
        //    }
        //}
        if self.indicates(other) {
            let sl = self.0.len();
            let ol = other.0.len();

            if sl + 1 == ol && self.0[sl - 1] == other.0[ol - 2] {
                if other.0[ol - 1] == T::new(0) {
                    Some(IndicationClass::Direct)
                } else {
                    Some(IndicationClass::Indirect)
                }
            } else {
                Some(IndicationClass::Reduction)
            }
        } else {
            None
        }
    }

    fn tightest_classification<I>(&self, nodes: I) -> Option<IndicationClass>
    where
        I: IntoIterator<Item = Index<T>>,
    {
        todo!();
        // let mut indicates_indirectly = false;

        // let result = nodes.into_iter().any(|r| match self.classify(&r) {
        //     Some(IndicationClass::Direct) => true,
        //     Some(IndicationClass::Indirect) => {
        //         indicates_indirectly = true;
        //         false
        //     }
        //     None => false,
        // });

        // if result {
        //     Some(IndicationClass::Direct)
        // } else if indicates_indirectly {
        //     Some(IndicationClass::Indirect)
        // } else {
        //     None
        // }
    }

    // fn indication_indicates_one_of<I>(&self, nodes: I) -> bool
    // where
    //     I: IntoIterator<Item = Index<T>>,
    // {

    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_ord_0() {
        let i0 = Index::<u32>::from(vec![0, 1, 2]);
        let i1 = Index::<u32>::from(vec![2, 1, 0]);

        assert_eq!(Ordering::Less, i0.cmp(&i1));
    }

    #[test]
    fn index_ord_1() {
        let i0 = Index::<u32>::from(vec![0, 1, 2]);
        let i1 = Index::<u32>::from(vec![2, 1, 0]);

        assert_eq!(Ordering::Greater, i1.cmp(&i0));
    }

    #[test]
    fn index_ord_2() {
        let i0 = Index::<u32>::from(vec![0, 1, 2]);
        let i1 = Index::<u32>::from(vec![0, 1, 2]);

        assert_eq!(Ordering::Equal, i0.cmp(&i1));
    }

    #[test]
    fn index_ord_3() {
        let i0 = Index::<u32>::from(vec![0, 1, 2]);
        let i1 = Index::<u32>::from(vec![0, 1, 2, 3]);

        assert_eq!(Ordering::Less, i0.cmp(&i1));
    }

    #[test]
    fn index_ord_4() {
        let i0 = Index::<u32>::from(vec![1, 1, 2]);
        let i1 = Index::<u32>::from(vec![0, 1, 2, 3]);

        assert_eq!(Ordering::Greater, i0.cmp(&i1));
    }

    #[test]
    fn index_ord_5() {
        let i0 = Index::<u32>::from(vec![10, 10, 10, 10]);
        let i1 = Index::<u32>::from(vec![10, 10, 10, 10, 10]);

        assert_eq!(Ordering::Less, i0.cmp(&i1));
    }

    #[test]
    fn index_indicates_0() {
        let i0 = Index::<u32>::from(vec![1]);
        let i1 = Index::<u32>::from(vec![1, 0]);

        assert!(i0.indicates(&i1));
        assert!(!i1.indicates(&i0));
    }

    #[test]
    fn index_indicates_1() {
        let i0 = Index::<u32>::from(vec![1]);
        let i1 = Index::<u32>::from(vec![1, 1]);

        assert!(i0.indicates(&i1));
        assert!(!i1.indicates(&i0));
    }

    #[test]
    fn index_indicates_2() {
        let i0 = Index::<u32>::from(vec![0]);
        let i1 = Index::<u32>::from(vec![1, 0]);

        assert!(i0.indicates(&i1));
        assert!(!i1.indicates(&i0));
    }

    #[test]
    fn index_indicates_3() {
        let i0 = Index::<u32>::from(vec![0]);
        let i1 = Index::<u32>::from(vec![1, 0]);

        assert!(i0.indicates(&i1));
        assert!(!i1.indicates(&i0));
    }

    #[test]
    fn index_indicates_4() {
        let i0 = Index::<u32>::from(vec![0]);
        let i1 = Index::<u32>::from(vec![1]);

        assert!(!i0.indicates(&i1));
        assert!(!i1.indicates(&i0));
    }

    #[test]
    fn index_indicates_5() {
        let i0 = Index::<u32>::from(vec![1, 0]);
        let i1 = Index::<u32>::from(vec![1, 3, 1]);

        assert!(i0.indicates(&i1));
        assert!(!i1.indicates(&i0));
    }

    #[test]
    fn index_indicate_0() {
        let i = Index::<u32>::from(vec![1, 0]);
        let expected = Index::<u32>::from(vec![1, 0, 5]);
        assert_eq!(i.indicate(5), expected);
    }

    #[test]
    fn classify_0() {
        let i0 = Index::<u32>::from(vec![1, 0]);
        let i1 = Index::<u32>::from(vec![1, 0, 0]);
        assert_eq!(i0.classify(&i1), Some(IndicationClass::Direct))
    }

    #[test]
    fn classify_1() {
        let i0 = Index::<u32>::from(vec![1, 0]);
        let i1 = Index::<u32>::from(vec![1, 0, 1]);
        assert_eq!(i0.classify(&i1), Some(IndicationClass::Indirect))
    }

    #[test]
    fn classify_2() {
        let i0 = Index::<u32>::from(vec![1, 0]);
        let i1 = Index::<u32>::from(vec![1, 1, 0]);
        assert_eq!(i0.classify(&i1), Some(IndicationClass::Reduction));
    }

    #[test]
    fn classify_3() {
        let i0 = Index::<u32>::from(vec![1, 0]);
        let i1 = Index::<u32>::from(vec![1]);
        assert_eq!(i0.classify(&i1), None);
    }

    #[test]
    fn classify_4() {
        let i0 = Index::<u32>::from(vec![1, 0]);
        let i1 = Index::<u32>::from(vec![2, 0]);
        assert_eq!(i0.classify(&i1), None);
    }

    #[test]
    fn classify_5() {
        // TODO: not sure about this test.
        let i0 = Index::<u32>::from(vec![1, 0]);
        let i1 = Index::<u32>::from(vec![1, 0]);
        assert_eq!(i0.classify(&i1), None);
    }

    #[test]
    fn copy_instructor_iterator_0() {
        use CopyInstruction::*;
        let i = Index::<u32>::from(vec![0]);
        let v = vec![vec![1, 1], vec![3, 2]];
        let vs = v.iter().map(|v| Index::<u32>::from(v.clone())).collect();
        let instructions = CopyInstructor::new(i, &vs, 0)
            .into_iter()
            .collect::<Vec<_>>();
        let expected: Vec<(Index<u32>, CopyInstruction)> = vec![
            (vec![0], Indicate),
            (vec![1], Follow),
            (vec![2], Indicate),
            (vec![3], Follow),
            (vec![4], Extend),
        ]
        .iter()
        .map(|(v, i)| (Index::<u32>::from(v.clone()), *i))
        .collect();
        assert_eq!(instructions, expected);
    }

    #[test]
    fn copy_instructor_iterator_1() {
        use CopyInstruction::*;
        let i = Index::<u32>::from(vec![0]);
        let v = vec![vec![1, 1], vec![2, 0]];
        let vs = v.iter().map(|v| Index::<u32>::from(v.clone())).collect();
        let instructions = CopyInstructor::new(i, &vs, 0)
            .into_iter()
            .collect::<Vec<_>>();
        let expected: Vec<(Index<u32>, CopyInstruction)> = vec![
            (vec![0], Indicate),
            (vec![1], Follow),
            (vec![2], Replace),
            (vec![3], Extend),
        ]
        .iter()
        .map(|(v, i)| (Index::<u32>::from(v.clone()), *i))
        .collect();
        assert_eq!(instructions, expected);
    }

    #[test]
    fn copy_instructor_iterator_2() {
        use CopyInstruction::*;
        let i = Index::<u32>::from(vec![3, 0]);
        let v = vec![vec![1, 1], vec![3, 2]];
        let vs = v.iter().map(|v| Index::<u32>::from(v.clone())).collect();
        let instructions = CopyInstructor::new(i, &vs, 1)
            .into_iter()
            .collect::<Vec<_>>();
        let expected: Vec<(Index<u32>, CopyInstruction)> = vec![
            (vec![3, 0], Indicate),
            (vec![3, 1], Indicate),
            (vec![3, 2], Replace),
            (vec![3, 3], Extend),
        ]
        .iter()
        .map(|(v, i)| (Index::<u32>::from(v.clone()), *i))
        .collect();
        assert_eq!(instructions, expected);
    }

    // #[test]
    // fn tightest_classification_0() {
    //     let i = Index::<u32>::from(vec![0]);
    //     let v = vec![vec![1], vec![0, 1]];
    //     let vs = v.iter().map(|v| Index::<u32>::from(v.clone()));
    //     assert_eq!(
    //         Some(IndicationClass::Indirect),
    //         i.tightest_classification(vs)
    //     );
    // }

    // #[test]
    // fn tightest_classification_1() {
    //     let i = Index::<u32>::from(vec![0]);
    //     let v = vec![vec![1], vec![0, 0], vec![0, 1]];
    //     let vs = v.iter().map(|v| Index::<u32>::from(v.clone()));
    //     assert_eq!(Some(IndicationClass::Direct), i.tightest_classification(vs));
    // }

    // #[test]
    // fn tightest_classification_2() {
    //     let i = Index::<u32>::from(vec![1]);
    //     let v = vec![vec![1], vec![0, 0], vec![0, 1]];
    //     let vs = v.iter().map(|v| Index::<u32>::from(v.clone()));
    //     assert_eq!(None, i.tightest_classification(vs));
    // }

    // fn magic_0() {
    //     let i = Index::<u32>::from(vec![1]);
    //     let v = vec![vec![1], vec![0, 0], vec![0, 1]];
    //     let vs = v.iter().map(|v| Index::<u32>::from(v.clone()));
    //     assert_eq!(None, i.tightest_classification(vs));
    // }
}
