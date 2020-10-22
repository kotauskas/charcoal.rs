use super::*;

#[test]
fn basic() {
    let mut tree: BinaryTree<u64> = BinaryTree::new(1987_u64);
    tree.root_mut().make_branch((83, Some(87))).unwrap();
    
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