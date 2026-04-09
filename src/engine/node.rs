use super::internal_node::InternalNode;
use super::leaf_node::LeafNode;

#[derive(Debug, Clone)]
pub(crate) enum Node {
    Internal(InternalNode),
    Leaf(LeafNode),
}
