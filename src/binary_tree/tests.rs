use super::*;
use arrayvec::ArrayVec;

#[test]
fn basic() {
    let mut tree: BinaryTree<u64> = BinaryTree::new(1987_u64);
    tree.root_mut().set_children(ArrayVec::from([83, 87]));
    
    let left_child_val = tree.root().left_child().as_ref().map(NodeRef::value);
    let right_child_val = tree.root().right_child().as_ref().map(NodeRef::value);
    assert_eq!(
        left_child_val,
        Some(NodeValue::Leaf(&83)),
    );
    assert_eq!(
        right_child_val,
        Some(NodeValue::Leaf(&87)),
    );
}