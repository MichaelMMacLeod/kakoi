use bitvec::prelude::*;
// use std::cmp::Ordering;

pub trait Index: Clone {
    // either self or a reduction of self directly or indirectly indicates it
    fn indicates(&self, other: &Self) -> bool;

    // self directly indicates it
    fn directly_indicates(&self, other: &Self) -> bool;

    // self directly indicates something that indicates it
    fn indirectly_indicates(&self, other: &Self) -> bool;

    fn indicate(&self) -> Self {
        let mut c = self.clone();
        c.indicate_mut();
        c
    }

    fn indicate_mut(&mut self);

    fn reduce(&self) -> Self {
        let mut c = self.clone();
        c.reduce_mut();
        c
    }

    fn reduce_mut(&mut self);
}

// Indication edges are represented as '1's (true), extension edges are
// represented as '0's (false).
impl Index for BitVec {
    fn indicates(&self, other: &Self) -> bool {
        // self:  <prefix>
        // other: <prefix><zero or more 0s>1<anything>
        self.len() < other.len()
            && self.iter().zip(other.iter()).all(|(s, o)| s == o)
            && other.iter().skip(self.len()).any(|v| v == true)
    }

    fn directly_indicates(&self, other: &Self) -> bool {
        // self:  <prefix>
        // other: <prefix>1
        self.len() + 1 == other.len() && other.last().unwrap() == true && self.indicates(other)
    }

    fn indirectly_indicates(&self, other: &Self) -> bool {
        self.indicates(other) && other[self.len()] == true
        // self:  <prefix>
        // other: <prefix>1<anything>
    }

    fn reduce_mut(&mut self) {
        self.push(false);
    }

    fn indicate_mut(&mut self) {
        self.push(true);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn indicates_0() {
        assert!(!bitvec![].indicates(&bitvec![0]));
    }

    #[test]
    fn indicates_1() {
        assert!(bitvec![].indicates(&bitvec![1]));
    }

    #[test]
    fn indicates_2() {
        assert!(!bitvec![].indicates(&bitvec![0, 0]));
    }

    #[test]
    fn indicates_3() {
        assert!(bitvec![].indicates(&bitvec![0, 1]));
    }

    #[test]
    fn indicates_4() {
        assert!(bitvec![].indicates(&bitvec![1, 0]));
    }

    #[test]
    fn indicates_5() {
        assert!(bitvec![].indicates(&bitvec![1, 1]));
    }

    #[test]
    fn indicates_6() {
        assert!(!bitvec![].indicates(&bitvec![]));
    }

    #[test]
    fn indicates_7() {
        assert!(!bitvec![0].indicates(&bitvec![0]));
    }

    #[test]
    fn indicates_8() {
        assert!(bitvec![0].indicates(&bitvec![0, 1]));
    }

    #[test]
    fn indicates_9() {
        assert!(
            bitvec![1, 0, 0, 1, 1, 0, 0].indicates(&bitvec![1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0])
        );
    }

    #[test]
    fn directly_indicates_0() {
        assert!(bitvec![].directly_indicates(&bitvec![1]));
    }

    #[test]
    fn directly_indicates_1() {
        assert!(!bitvec![].directly_indicates(&bitvec![]));
    }

    #[test]
    fn directly_indicates_2() {
        assert!(bitvec![0].directly_indicates(&bitvec![0, 1]));
    }

    #[test]
    fn directly_indicates_3() {
        assert!(bitvec![0, 0, 1, 1, 0, 1].directly_indicates(&bitvec![0, 0, 1, 1, 0, 1, 1]));
    }

    #[test]
    fn directly_indicates_4() {
        assert!(!bitvec![0, 0, 1, 1, 0, 1].directly_indicates(&bitvec![0, 0, 1, 1, 0, 1, 0, 1]));
    }

    #[test]
    fn directly_indicates_5() {
        assert!(!bitvec![0].directly_indicates(&bitvec![1]));
    }

    #[test]
    fn indirectly_indicates_0() {
        assert!(bitvec![].indirectly_indicates(&bitvec![1]));
    }

    #[test]
    fn indirectly_indicates_1() {
        assert!(bitvec![].indirectly_indicates(&bitvec![1, 0]));
    }

    #[test]
    fn indirectly_indicates_2() {
        assert!(bitvec![].indirectly_indicates(&bitvec![1, 1]));
    }

    #[test]
    fn indirectly_indicates_3() {
        assert!(!bitvec![].indirectly_indicates(&bitvec![0]));
    }

    #[test]
    fn indirectly_indicates_4() {
        assert!(!bitvec![].indirectly_indicates(&bitvec![0, 1]));
    }

    #[test]
    fn indirectly_indicates_5() {
        assert!(bitvec![0, 0, 1, 1, 0].indirectly_indicates(&bitvec![0, 0, 1, 1, 0, 1, 0, 0, 1]));
    }

    #[test]
    fn indirectly_indicates_6() {
        assert!(!bitvec![0, 0, 1, 1, 0].indirectly_indicates(&bitvec![0, 0, 1, 1, 0, 0, 1]));
    }
}
