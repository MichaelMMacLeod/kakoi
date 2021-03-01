use petgraph::graph::IndexType;
use std::cmp::Ordering;
use std::convert::From;
use Ordering::{Equal, Greater, Less};

#[derive(Eq)]
pub struct Index<T: IndexType>(Vec<T>);

impl<T: IndexType> From<Vec<T>> for Index<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T: IndexType> Ord for Index<T> {
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

impl<T: IndexType> PartialOrd for Index<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: IndexType> PartialEq for Index<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Equal
    }
}

impl<T: IndexType> Index<T> {
    fn indicates(&self, other: &Self) -> bool {
        self.0.len() < other.0.len() && self.0.iter().zip(other.0.iter()).all(|(s, o)| s <= o)
    }
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
}
