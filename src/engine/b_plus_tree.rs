const DEFAULT_ORDER: usize = 4;

#[derive(Debug, Clone)]
enum Node {
    Internal(InternalNode),
    Leaf(LeafNode),
}

#[derive(Debug, Clone)]
struct InternalNode {
    // Separator keys, children.len() == keys.len() + 1
    keys: Vec<i64>,
    children: Vec<usize>,
}

#[derive(Debug, Clone)]
struct LeafNode {
    keys: Vec<i64>,
    values: Vec<Vec<u8>>,
    next: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct BPlusTree {
    order: usize,
    root: usize,
    nodes: Vec<Node>,
    len: usize,
}

impl BPlusTree {
    pub fn new() -> Self {
        Self::with_order(DEFAULT_ORDER)
    }

    pub fn with_order(order: usize) -> Self {
        assert!(order >= 3, "B+ tree order must be >= 3");

        let root = Node::Leaf(LeafNode {
            keys: Vec::new(),
            values: Vec::new(),
            next: None,
        });

        Self {
            order,
            root: 0,
            nodes: vec![root],
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(&mut self, key: i64, value: Vec<u8>) {
        let split = self.insert_recursive(self.root, key, value);

        if let Some((promoted_key, right_id)) = split {
            let old_root = self.root;
            let new_root = InternalNode {
                keys: vec![promoted_key],
                children: vec![old_root, right_id],
            };
            self.nodes.push(Node::Internal(new_root));
            self.root = self.nodes.len() - 1;
        }
    }

    pub fn get(&self, key: i64) -> Option<&[u8]> {
        let leaf_id = self.find_leaf_id(key);

        match &self.nodes[leaf_id] {
            Node::Leaf(leaf) => leaf
                .keys
                .binary_search(&key)
                .ok()
                .map(|idx| leaf.values[idx].as_slice()),
            Node::Internal(_) => None,
        }
    }

    pub fn delete(&mut self, key: i64) -> Option<Vec<u8>> {
        let leaf_id = self.find_leaf_id(key);

        match &mut self.nodes[leaf_id] {
            Node::Leaf(leaf) => match leaf.keys.binary_search(&key) {
                Ok(pos) => {
                    leaf.keys.remove(pos);
                    self.len -= 1;
                    Some(leaf.values.remove(pos))
                }
                Err(_) => None,
            },
            Node::Internal(_) => None,
        }
    }

    pub fn range_query(&self, start: i64, end: i64) -> Vec<(i64, Vec<u8>)> {
        if start > end {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current = self.find_leaf_id(start);

        loop {
            let leaf = match &self.nodes[current] {
                Node::Leaf(leaf) => leaf,
                Node::Internal(_) => break,
            };

            for (k, v) in leaf.keys.iter().zip(leaf.values.iter()) {
                if *k < start {
                    continue;
                }
                if *k > end {
                    return result;
                }
                result.push((*k, v.clone()));
            }

            match leaf.next {
                Some(next_id) => current = next_id,
                None => break,
            }
        }

        result
    }

    fn max_keys(&self) -> usize {
        self.order - 1
    }

    fn insert_recursive(
        &mut self,
        node_id: usize,
        key: i64,
        value: Vec<u8>,
    ) -> Option<(i64, usize)> {
        let is_leaf = matches!(self.nodes[node_id], Node::Leaf(_));

        if is_leaf {
            self.insert_into_leaf(node_id, key, value)
        } else {
            self.insert_into_internal(node_id, key, value)
        }
    }

    fn insert_into_leaf(
        &mut self,
        leaf_id: usize,
        key: i64,
        value: Vec<u8>,
    ) -> Option<(i64, usize)> {
        let max_keys = self.max_keys();

        let leaf = match &mut self.nodes[leaf_id] {
            Node::Leaf(leaf) => leaf,
            Node::Internal(_) => return None,
        };

        match leaf.keys.binary_search(&key) {
            Ok(pos) => {
                // Update existing key.
                leaf.values[pos] = value;
                return None;
            }
            Err(pos) => {
                leaf.keys.insert(pos, key);
                leaf.values.insert(pos, value);
                self.len += 1;
            }
        }

        if leaf.keys.len() <= max_keys {
            return None;
        }

        let split_pos = leaf.keys.len() / 2;

        let right_keys = leaf.keys.split_off(split_pos);
        let right_values = leaf.values.split_off(split_pos);
        let promoted_key = right_keys[0];
        let old_next = leaf.next;

        let right_leaf = LeafNode {
            keys: right_keys,
            values: right_values,
            next: old_next,
        };

        self.nodes.push(Node::Leaf(right_leaf));
        let right_id = self.nodes.len() - 1;

        if let Node::Leaf(left_leaf) = &mut self.nodes[leaf_id] {
            left_leaf.next = Some(right_id);
        }

        Some((promoted_key, right_id))
    }

    fn insert_into_internal(
        &mut self,
        node_id: usize,
        key: i64,
        value: Vec<u8>,
    ) -> Option<(i64, usize)> {
        let child_index = match &self.nodes[node_id] {
            Node::Internal(internal) => child_index_for_key(&internal.keys, key),
            Node::Leaf(_) => return None,
        };

        let child_id = match &self.nodes[node_id] {
            Node::Internal(internal) => internal.children[child_index],
            Node::Leaf(_) => return None,
        };

        let split = self.insert_recursive(child_id, key, value);

        let (promoted_key, right_child_id) = split?;

        let max_keys = self.max_keys();

        let internal = match &mut self.nodes[node_id] {
            Node::Internal(internal) => internal,
            Node::Leaf(_) => return None,
        };

        let insert_pos = child_index_for_key(&internal.keys, promoted_key);
        internal.keys.insert(insert_pos, promoted_key);
        internal.children.insert(insert_pos + 1, right_child_id);

        if internal.keys.len() <= max_keys {
            return None;
        }

        let mid = internal.keys.len() / 2;
        let promoted = internal.keys[mid];

        let right_keys = internal.keys.split_off(mid + 1);
        let right_children = internal.children.split_off(mid + 1);
        internal.keys.pop();

        let right_internal = InternalNode {
            keys: right_keys,
            children: right_children,
        };

        self.nodes.push(Node::Internal(right_internal));
        let right_id = self.nodes.len() - 1;

        Some((promoted, right_id))
    }

    fn find_leaf_id(&self, key: i64) -> usize {
        let mut current = self.root;

        loop {
            match &self.nodes[current] {
                Node::Leaf(_) => return current,
                Node::Internal(internal) => {
                    let idx = child_index_for_key(&internal.keys, key);
                    current = internal.children[idx];
                }
            }
        }
    }
}

fn child_index_for_key(keys: &[i64], key: i64) -> usize {
    match keys.binary_search(&key) {
        Ok(i) => i + 1,
        Err(i) => i,
    }
}

#[cfg(test)]
mod tests {
    use super::BPlusTree;

    fn v(n: i64) -> Vec<u8> {
        format!("v{n}").into_bytes()
    }

    #[test]
    fn insert_and_get() {
        let mut tree = BPlusTree::with_order(4);
        tree.insert(10, v(10));
        tree.insert(20, v(20));
        tree.insert(15, v(15));

        assert_eq!(tree.get(10), Some("v10".as_bytes()));
        assert_eq!(tree.get(15), Some("v15".as_bytes()));
        assert_eq!(tree.get(99), None);
        assert_eq!(tree.len(), 3);
    }

    #[test]
    fn update_existing_key() {
        let mut tree = BPlusTree::new();
        tree.insert(7, b"old".to_vec());
        tree.insert(7, b"new".to_vec());

        assert_eq!(tree.get(7), Some("new".as_bytes()));
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn split_and_range_query() {
        let mut tree = BPlusTree::with_order(4);
        for i in 1..=20 {
            tree.insert(i, v(i));
        }

        let got: Vec<i64> = tree
            .range_query(6, 12)
            .into_iter()
            .map(|(k, _)| k)
            .collect();

        assert_eq!(got, vec![6, 7, 8, 9, 10, 11, 12]);
    }

    #[test]
    fn delete_key() {
        let mut tree = BPlusTree::new();
        tree.insert(1, v(1));
        tree.insert(2, v(2));

        let deleted = tree.delete(1);
        assert_eq!(deleted, Some(v(1)));
        assert_eq!(tree.get(1), None);
        assert_eq!(tree.get(2), Some("v2".as_bytes()));
        assert_eq!(tree.len(), 1);
    }
}
