use core::{fmt::Debug, hint};
use crate::{
    storage::Storage,
    traversal::{
        Traversable,
        TraversableMut,
        VisitorDirection,
        CursorResult,
        CursorDirectionError,
    },
    util::{ArrayMap, unreachable_debugchecked},
    NodeValue,
    TryRemoveBranchError,
    TryRemoveLeafError,
    TryRemoveChildrenError,
};
use super::{Octree, Node, NodeRef, NodeRefMut, PackedChildren};

impl<B, L, K, S> Traversable for Octree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    type Leaf = L;
    type Branch = B;
    type Cursor = K;

    fn advance_cursor<V>(
        &self,
        cursor: Self::Cursor,
        direction: VisitorDirection<Self::Cursor, V>,
    ) -> CursorResult<Self::Cursor> {
        // Create the error in advance to avoid duplication
        let error = CursorDirectionError {
            previous_state: cursor.clone(),
        };
        let node = NodeRef::new_raw(self, cursor)
            .expect("the node specified by the cursor does not exist");
        match direction {
            VisitorDirection::Parent => node.parent().ok_or(error).map(NodeRef::into_raw_key),
            VisitorDirection::NextSibling => {
                node.child_index()
                    .map(|child_index| {
                        let parent = node.parent().unwrap_or_else(|| unsafe {
                            unreachable_debugchecked("parent nodes cannot be leaves")
                        });
                        parent
                            .nth_child(child_index)
                            .unwrap_or_else(|| unsafe {
                                // SAFETY: the previous unreachable_debugchecked checked for this
                                hint::unreachable_unchecked()
                            })
                            .into_raw_key()
                    })
                    .ok_or(error)
            }
            VisitorDirection::Child(num) => {
                let num = if num <= 7 {
                    num as u8
                } else {
                    return Err(error);
                };
                node.nth_child(num).map(NodeRef::into_raw_key).ok_or(error)
            }
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
        if node_ref.is_branch() {
            8
        } else {
            0
        }
    }
    #[track_caller]
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor> {
        if child_num < 8 {
            let node_ref = NodeRef::new_raw(self, cursor.clone())
                .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
            node_ref
                .nth_child(child_num as u8)
                .map(NodeRef::into_raw_key)
        } else {
            None
        }
    }
}
impl<B, L, K, S> TraversableMut for Octree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    const CAN_REMOVE_INDIVIDUAL_CHILDREN: bool = false;
    const CAN_PACK_CHILDREN: bool = true;
    type PackedChildren = PackedChildren<L>;

    #[track_caller]
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
        _cursor: &Self::Cursor,
        _branch_to_leaf: BtL,
    ) -> Result<Self::Leaf, TryRemoveLeafError> {
        Err(TryRemoveLeafError::CannotRemoveIndividualChildren)
    }
    fn try_remove_branch_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        _cursor: &Self::Cursor,
        _branch_to_leaf: BtL,
        _collector: C,
    ) -> Result<Self::Branch, TryRemoveBranchError> {
        Err(TryRemoveBranchError::CannotRemoveIndividualChildren)
    }
    #[track_caller]
    fn try_remove_children_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
        mut collector: C,
    ) -> Result<(), TryRemoveChildrenError> {
        let mut node_ref = NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        node_ref.try_remove_children_with(branch_to_leaf).map(|x| {
            x.array_map(|e| collector(e));
        })
    }
    fn try_remove_branch<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        _cursor: &Self::Cursor,
        _branch_to_leaf: BtL,
    ) -> Result<(Self::Branch, Self::PackedChildren), TryRemoveBranchError> {
        Err(TryRemoveBranchError::CannotRemoveIndividualChildren)
    }
    #[track_caller]
    fn try_remove_children<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
    ) -> Result<Self::PackedChildren, TryRemoveChildrenError> {
        let mut node_ref = NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        node_ref
            .try_remove_children_with(branch_to_leaf)
            .map(Into::into)
    }
}
