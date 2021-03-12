use crate::{
    storage::Storage,
    traversal::{Traversable, TraversableMut, VisitorDirection, CursorDirectionError},
    util::unreachable_debugchecked,
    NodeValue,
    TryRemoveBranchError,
    TryRemoveLeafError,
    TryRemoveChildrenError,
};
use arrayvec::ArrayVec;
use super::*;

impl<B, L, K, S> Traversable for BinaryTree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    type Branch = B;
    type Leaf = L;
    type Cursor = K;

    fn advance_cursor<V>(
        &self,
        cursor: Self::Cursor,
        direction: VisitorDirection<Self::Cursor, V>,
    ) -> Result<Self::Cursor, CursorDirectionError<Self::Cursor>> {
        // Create the error in advance to avoid duplication
        let error = CursorDirectionError {
            previous_state: cursor.clone(),
        };
        let node = NodeRef::new_raw(self, cursor)
            .expect("the node specified by the cursor does not exist");
        match direction {
            VisitorDirection::Parent => node.parent().ok_or(error).map(NodeRef::into_raw_key),
            VisitorDirection::NextSibling => {
                if node.is_left_child() == Some(true) {
                    node.parent()
                        .unwrap_or_else(|| unsafe {
                            unreachable_debugchecked("parent nodes cannot be leaves")
                        })
                        .right_child()
                        .map(NodeRef::into_raw_key)
                        .ok_or(error)
                } else {
                    Err(error)
                }
            }
            VisitorDirection::Child(num) => match num {
                0 => node.left_child().ok_or(error).map(NodeRef::into_raw_key),
                1 => node.right_child().ok_or(error).map(NodeRef::into_raw_key),
                _ => Err(error),
            },
            VisitorDirection::SetTo(new_cursor) => {
                if self.storage.contains_key(&new_cursor) {
                    Ok(new_cursor)
                } else {
                    // Do not allow returning invalid cursors, as those will cause panicking
                    Err(error)
                }
            }
            VisitorDirection::Stop(..) => Err(error),
        }
    }
    fn cursor_to_root(&self) -> Self::Cursor {
        self.root.clone()
    }
    #[track_caller]
    fn value_of(&self, cursor: &Self::Cursor) -> NodeValue<&'_ Self::Branch, &'_ Self::Leaf> {
        let node_ref = NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        node_ref.value()
    }
    #[track_caller]
    fn parent_of(&self, cursor: &Self::Cursor) -> Option<Self::Cursor> {
        let node_ref = NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        node_ref.parent().map(NodeRef::into_raw_key)
    }
    #[track_caller]
    fn num_children_of(&self, cursor: &Self::Cursor) -> usize {
        let node_ref = NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        if node_ref.is_full_branch() {
            2
        } else if node_ref.is_branch() {
            1
        } else {
            0
        }
    }
    #[track_caller]
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor> {
        let node_ref = NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        match child_num {
            0 => node_ref.left_child().map(NodeRef::into_raw_key),
            1 => node_ref.right_child().map(NodeRef::into_raw_key),
            _ => None,
        }
    }
}
impl<B, L, K, S> TraversableMut for BinaryTree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    const CAN_REMOVE_INDIVIDUAL_CHILDREN: bool = true;
    const CAN_PACK_CHILDREN: bool = true;
    type PackedChildren = ArrayVec<[Self::Leaf; 2]>;
    fn value_mut_of(
        &mut self,
        cursor: &Self::Cursor,
    ) -> NodeValue<&'_ mut Self::Branch, &'_ mut Self::Leaf> {
        self.storage
            .get_mut(cursor)
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .value
            .as_mut()
            .into_value()
    }
    fn try_remove_leaf<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
    ) -> Result<Self::Leaf, TryRemoveLeafError> {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_leaf_with(branch_to_leaf)
    }
    #[allow(clippy::type_complexity)]
    fn try_remove_branch_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
        mut collector: C,
    ) -> Result<Self::Branch, TryRemoveBranchError> {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_branch_with(branch_to_leaf)
            .map(|x| {
                collector(x.1);
                if let Some(right_child) = x.2 {
                    collector(right_child);
                }
                x.0
            })
    }
    #[allow(clippy::type_complexity)]
    fn try_remove_children_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
        mut collector: C,
    ) -> Result<(), TryRemoveChildrenError> {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_children_with(branch_to_leaf)
            .map(|x| {
                collector(x.0);
                if let Some(right_child) = x.1 {
                    collector(right_child);
                }
            })
    }
    #[allow(clippy::type_complexity)]
    fn try_remove_branch<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
    ) -> Result<(Self::Branch, Self::PackedChildren), TryRemoveBranchError> {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_branch_with(branch_to_leaf)
            .map(|x| {
                let mut children = ArrayVec::new();
                children.push(x.1);
                if let Some(right_child) = x.2 {
                    children.push(right_child);
                }
                (x.0, children)
            })
    }
    #[allow(clippy::type_complexity)]
    fn try_remove_children<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
    ) -> Result<Self::PackedChildren, TryRemoveChildrenError> {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_children_with(branch_to_leaf)
            .map(|x| {
                let mut children = ArrayVec::new();
                children.push(x.0);
                if let Some(right_child) = x.1 {
                    children.push(right_child);
                }
                children
            })
    }
}
