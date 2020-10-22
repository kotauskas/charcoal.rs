//! Trees which allow nodes to have either zero children or exactly **8**, most often used to partition a 3D space by recursively subdividing it into eight octants.
//!
//! The [Wikipedia article] on binary trees covers their use cases and specifics in more detail.
//!
//! [Wikipedia article]: https://en.wikipedia.org/wiki/Octree " "

use core::{
    fmt::Debug,
    iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator},
    borrow::{Borrow, BorrowMut},
};
use crate::{
    storage::{Storage, ListStorage, DefaultStorage, SparseStorage, SparseStorageSlot},
    traversal::{Traversable, TraversableMut, VisitorDirection, CursorResult, CursorDirectionError},
    NodeValue,
    TryRemoveBranchError,
    TryRemoveLeafError,
    TryRemoveChildrenError,
};
use arrayvec::{ArrayVec, IntoIter as ArrayVecIntoIter};

mod node;
mod node_ref;
#[cfg(test)]
mod tests;

use node::NodeData;
pub use node::Node;
pub use node_ref::{NodeRef, NodeRefMut};

/// An octree.
///
/// See the [module-level documentation] for more.
///
/// [module-level documentation]: index.html " "
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Octree<B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    storage: S,
    root: K,
}
impl<B, L, K, S> Octree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates an octree with the specified value for the root node.
    #[inline(always)]
    pub fn new(root: L) -> Self {
        let mut storage = S::new();
        let root = storage.add(unsafe {
            // SAFETY: there isn't a root there yet
            Node::root(root)
        });
        Self { storage, root }
    }
    /// Creates an empty octree with the specified capacity for the storage.
    ///
    /// # Panics
    /// The storage may panic if it has fixed capacity and the specified value does not match it.
    #[inline(always)]
    pub fn with_capacity(capacity: usize, root: L) -> Self {
        let mut storage = S::with_capacity(capacity);
        let root = storage.add(unsafe {
            // SAFETY: as above
            Node::root(root)
        });
        Self { storage, root }
    }
}
impl<B, L, S> Octree<B, L, usize, SparseStorage<Node<B, L, usize>, S>>
where
    S: ListStorage<Element = SparseStorageSlot<Node<B, L, usize>>>,
{
    /// Removes all holes from the sparse storage.
    ///
    /// Under the hood, this uses `defragment_and_fix`. It's not possible to defragment without fixing the indicies, as that might cause undefined behavior.
    #[inline(always)]
    pub fn defragment(&mut self) {
        self.storage.defragment_and_fix()
    }
    /// Returns the number of holes in the storage. This operation returns immediately instead of looping through the entire storage, since the sparse storage automatically tracks the number of holes it creates and destroys.
    #[inline(always)]
    pub fn num_holes(&self) -> usize {
        self.storage.num_holes()
    }
    /// Returns `true` if there are no holes in the storage, `false` otherwise. This operation returns immediately instead of looping through the entire storage, since the sparse storage automatically tracks the number of holes it creates and destroys.
    #[inline(always)]
    pub fn is_dense(&self) -> bool {
        self.storage.is_dense()
    }
}

impl<B, L, K, S> Traversable for Octree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    type Leaf = L;
    type Branch = B;
    type Cursor = K;

    #[inline]
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
            VisitorDirection::NextSibling => todo!(), // TODO
            VisitorDirection::Child(num) => {
                let num = if num <= 7 {
                    num as u8
                } else {
                    return Err(error);
                };
                node.nth_child(num).map(NodeRef::into_raw_key).ok_or(error)
            },
            VisitorDirection::SetTo(new_cursor) => {
                if self.storage.contains_key(&new_cursor) {
                    Ok(new_cursor)
                } else {
                    // Do not allow returning invalid cursors, as those will cause panicking
                    Err(error)
                }
            },
            VisitorDirection::Stop(..) => Err(error),
        }
    }
    #[inline(always)]
    fn cursor_to_root(&self) -> Self::Cursor {
        self.root.clone()
    }
    #[inline]
    #[track_caller]
    fn value_of(&self, cursor: &Self::Cursor) -> NodeValue<&'_ Self::Branch, &'_ Self::Leaf> {
        let node_ref = NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        node_ref.value()
    }
    #[inline]
    #[track_caller]
    fn parent_of(&self, cursor: &Self::Cursor) -> Option<Self::Cursor> {
        let node_ref = NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        node_ref.parent().map(NodeRef::into_raw_key)
    }
    #[inline]
    #[track_caller]
    fn num_children_of(&self, cursor: &Self::Cursor) -> usize {
        let node_ref = NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        if node_ref.is_branch() {
            8
        }  else {
            0
        }
    }
    #[inline]
    #[track_caller]
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor> {
        if child_num < 8 {
            let node_ref = NodeRef::new_raw(self, cursor.clone())
               .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
            node_ref.nth_child(child_num as u8).map(NodeRef::into_raw_key)
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
    type PackedChildren = PackedChildren<L>;

    #[inline]
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
    #[inline(always)]
    fn try_remove_leaf_with<F: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        _cursor: &Self::Cursor,
        _f: F,
    ) -> Result<Self::Leaf, TryRemoveLeafError> {
        Err(TryRemoveLeafError::CannotRemoveIndividualChildren)
    }
    #[inline(always)]
    fn try_remove_branch_with<F: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        _cursor: &Self::Cursor,
        _f: F,
    ) -> Result<(Self::Branch, Self::PackedChildren), TryRemoveBranchError> {
        Err(TryRemoveBranchError::CannotRemoveIndividualChildren)
    }
    #[inline]
    #[track_caller]
    fn try_remove_children_with<F: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        f: F,
    ) -> Result<Self::PackedChildren, TryRemoveChildrenError> {
        let mut node_ref = NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        node_ref.try_remove_children_with(f).map(Into::into)
    }
}

/// Packed leaf children nodes of an octree's branch node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PackedChildren<T> (pub [T; 8]);
impl<T> PackedChildren<T> {
    /// Returns the packed children as an array.
    #[inline(always)]
    #[allow(clippy::missing_const_for_fn)] // cannot drop at compile time smh
    pub fn into_inner(self) -> [T; 8] {
        self.0
    }
}
impl<T> Borrow<[T]> for PackedChildren<T> {
    #[inline(always)]
    fn borrow(&self) -> &[T] {
        &self.0
    }
}
impl<T> BorrowMut<[T]> for PackedChildren<T> {
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut self.0
    }
}
impl<T> IntoIterator for PackedChildren<T> {
    type Item = T;
    type IntoIter = PackedChildrenIter<T>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.into()
    }
}
impl<T> From<[T; 8]> for PackedChildren<T> {
    #[inline(always)]
    fn from(op: [T; 8]) -> Self {
        Self(op)
    }
}

/// An owned iterator over the elements of `PackedChildren`.
#[derive(Clone, Debug)]
pub struct PackedChildrenIter<T> (ArrayVecIntoIter<[T; 8]>);
impl<T> From<PackedChildren<T>> for PackedChildrenIter<T> {
    #[inline(always)]
    fn from(op: PackedChildren<T>) -> Self {
        Self(ArrayVec::from(op.0).into_iter())
    }
}
impl<T> Iterator for PackedChildrenIter<T> {
    type Item = T;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<T> DoubleEndedIterator for PackedChildrenIter<T> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
impl<T> ExactSizeIterator for PackedChildrenIter<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<T> FusedIterator for PackedChildrenIter<T> {}