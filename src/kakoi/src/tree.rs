use slotmap::SlotMap;

struct Node<K: slotmap::Key, D> {
    data: D,
    children: Vec<K>,
}

struct Tree<K: slotmap::Key, D> {
    slot_map: SlotMap<K, Node<K, D>>,
}

impl<K: slotmap::Key, D> Tree<K, D> {
    pub fn new() -> Self {
        Tree {
            slot_map: SlotMap::with_key(),
        }
    }

    pub fn insert_root(&mut self, data: D) -> K {
        self.slot_map.insert(Node {
            data,
            children: vec![],
        })
    }

    pub fn insert_child(&mut self, parent: K, data: D) -> K {
        let child_key = self.slot_map.insert(Node {
            data,
            children: vec![],
        });
        self.slot_map
            .get_mut(parent)
            .unwrap()
            .children
            .push(child_key);
        child_key
    }

    pub fn get(&self, key: K) -> Option<&D> {
        Some(&self.slot_map.get(key)?.data)
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut D> {
        Some(&mut self.slot_map.get_mut(key)?.data)
    }

    pub fn children(&self, parent: K) -> Option<&[K]> {
        Some(&self.slot_map.get(parent)?.children)
    }

    pub fn remove_root(&mut self, root: K) {
        let mut todo = vec![root];

        while let Some(key) = todo.pop() {
            self.slot_map.remove(key).map(|mut value| {
                todo.append(&mut value.children);
            });
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use slotmap::new_key_type;

    new_key_type! {
        pub struct TreeKey;
    }

    #[test]
    fn tree_0() {
        let mut tree: Tree<TreeKey, u32> = Tree::new();
        let root = tree.insert_root(0);
        assert_eq!(0, tree.get(root).copied().unwrap());
        let child_keys = (0..10)
            .into_iter()
            .map(|i| tree.insert_child(root, i))
            .collect::<Vec<_>>();
        for (&k, i) in child_keys.iter().zip(0..) {
            assert_eq!(i, tree.get(k).copied().unwrap());
        }
        for (k, i) in tree.children(root).unwrap().iter().copied().zip(0..) {
            assert_eq!(i, tree.get(k).copied().unwrap())
        }
        tree.remove_root(root);
        assert!(tree.get(root).is_none());
        for k in child_keys {
            assert!(tree.get(k).is_none());
        }
    }
}
