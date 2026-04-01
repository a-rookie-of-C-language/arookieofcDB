use crate::key_codec::KeyEncoding;

const DEFAULT_ORDER: usize = 4;

#[derive(Debug, Clone)]
enum Node {
    Internal(InternalNode),
    Leaf(LeafNode),
}

#[derive(Debug, Clone)]
struct InternalNode {
    // Separator keys, children.len() == keys.len() + 1
    keys: Vec<KeyEncoding>,
    children: Vec<usize>,
}

#[derive(Debug, Clone)]
struct LeafNode {
    keys: Vec<KeyEncoding>,
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

    pub fn insert(&mut self, key: KeyEncoding, value: Vec<u8>) {
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

    pub fn get(&self, key: &KeyEncoding) -> Option<&[u8]> {
        let leaf_id = self.find_leaf_id(key);

        match &self.nodes[leaf_id] {
            Node::Leaf(leaf) => leaf
                .keys
                .binary_search(key)
                .ok()
                .map(|idx| leaf.values[idx].as_slice()),
            Node::Internal(_) => None,
        }
    }

    pub fn delete(&mut self, key: &KeyEncoding) -> Option<Vec<u8>> {
        let leaf_id = self.find_leaf_id(key);

        match &mut self.nodes[leaf_id] {
            Node::Leaf(leaf) => match leaf.keys.binary_search(key) {
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

    pub fn range_query(&self, start: &KeyEncoding, end: &KeyEncoding) -> Vec<(KeyEncoding, Vec<u8>)> {
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
                if k < start {
                    continue;
                }
                if k > end {
                    return result;
                }
                result.push((k.clone(), v.clone()));
            }

            match leaf.next {
                Some(next_id) => current = next_id,
                None => break,
            }
        }

        result
    }

    pub fn entries(&self) -> Vec<(KeyEncoding, Vec<u8>)> {
        let mut current = self.root;
        while let Node::Internal(internal) = &self.nodes[current] {
            current = internal.children[0];
        }

        let mut out = Vec::new();
        loop {
            let leaf = match &self.nodes[current] {
                Node::Leaf(leaf) => leaf,
                Node::Internal(_) => break,
            };

            out.extend(
                leaf.keys
                    .iter()
                    .cloned()
                    .zip(leaf.values.iter().cloned()),
            );

            match leaf.next {
                Some(next_id) => current = next_id,
                None => break,
            }
        }

        out
    }

    fn max_keys(&self) -> usize {
        self.order - 1
    }

    fn insert_recursive(
        &mut self,
        node_id: usize,
        key: KeyEncoding,
        value: Vec<u8>,
    ) -> Option<(KeyEncoding, usize)> {
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
        key: KeyEncoding,
        value: Vec<u8>,
    ) -> Option<(KeyEncoding, usize)> {
        let max_keys = self.max_keys();

        let leaf = match &mut self.nodes[leaf_id] {
            Node::Leaf(leaf) => leaf,
            Node::Internal(_) => return None,
        };

        match leaf.keys.binary_search(&key) {
            Ok(pos) => {
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
        let promoted_key = right_keys[0].clone();
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
        key: KeyEncoding,
        value: Vec<u8>,
    ) -> Option<(KeyEncoding, usize)> {
        let child_index = match &self.nodes[node_id] {
            Node::Internal(internal) => child_index_for_key(&internal.keys, &key),
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

        let insert_pos = child_index_for_key(&internal.keys, &promoted_key);
        internal.keys.insert(insert_pos, promoted_key);
        internal.children.insert(insert_pos + 1, right_child_id);

        if internal.keys.len() <= max_keys {
            return None;
        }

        let mid = internal.keys.len() / 2;
        let promoted = internal.keys[mid].clone();

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

    fn find_leaf_id(&self, key: &KeyEncoding) -> usize {
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

fn child_index_for_key(keys: &[KeyEncoding], key: &KeyEncoding) -> usize {
    match keys.binary_search(key) {
        Ok(i) => i + 1,
        Err(i) => i,
    }
}

#[cfg(test)]
mod tests {
    use super::BPlusTree;
    use crate::key_codec::KeyEncoding;

    fn v(n: i64) -> Vec<u8> {
        format!("v{n}").into_bytes()
    }

    fn k(n: i64) -> KeyEncoding {
        KeyEncoding::Int(n)
    }

    #[test]
    fn insert_and_get() {
        let mut tree = BPlusTree::with_order(4);
        tree.insert(k(10), v(10));
        tree.insert(k(20), v(20));
        tree.insert(k(15), v(15));

        assert_eq!(tree.get(&k(10)), Some("v10".as_bytes()));
        assert_eq!(tree.get(&k(15)), Some("v15".as_bytes()));
        assert_eq!(tree.get(&k(99)), None);
        assert_eq!(tree.len(), 3);
    }

    #[test]
    fn update_existing_key() {
        let mut tree = BPlusTree::new();
        tree.insert(k(7), b"old".to_vec());
        tree.insert(k(7), b"new".to_vec());

        assert_eq!(tree.get(&k(7)), Some("new".as_bytes()));
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn split_and_range_query() {
        let mut tree = BPlusTree::with_order(4);
        for i in 1..=20 {
            tree.insert(k(i), v(i));
        }

        let got: Vec<i64> = tree
            .range_query(&k(6), &k(12))
            .into_iter()
            .map(|(k, _)| match k {
                KeyEncoding::Int(v) => v,
                _ => -1,
            })
            .collect();

        assert_eq!(got, vec![6, 7, 8, 9, 10, 11, 12]);
    }

    #[test]
    fn delete_key() {
        let mut tree = BPlusTree::new();
        tree.insert(k(1), v(1));
        tree.insert(k(2), v(2));

        let deleted = tree.delete(&k(1));
        assert_eq!(deleted, Some(v(1)));
        assert_eq!(tree.get(&k(1)), None);
        assert_eq!(tree.get(&k(2)), Some("v2".as_bytes()));
        assert_eq!(tree.len(), 1);
    }
}
