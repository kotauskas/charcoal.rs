//! Binary tree — a data structure which allows at most two children for its nodes.

use core::fmt::Debug;
use crate::{
    storage::{
        Storage, ListStorage,
        DefaultStorage,
        SparseStorage, SparseStorageSlot, SparseVec,
    },
    traversal::{
        Traversable, TraversableMut,
        VisitorDirection, CursorDirectionError,
    },
    NodeValue,
};

mod node;
use node::NodeData;
pub use node::Node;
mod node_ref;
pub use node_ref::{NodeRef, NodeRefMut};

/// A binary tree.
///
/// If all nodes of a binary tree have either 0 or 2 children, it's a **full binary tree**. This type supports both full and non-full trees.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BinaryTree<B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    storage: S,
    root: K,
}
impl<B, L, K, S> BinaryTree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    /// Creates an binary tree with the specified value for the root node.
    #[inline(always)]
    pub fn new(root: L) -> Self {
        let mut storage = S::new();
        let root = storage.add(unsafe {
            // SAFETY: there isn't a root there yet
            Node::root(root)
        });
        Self {storage, root}
    }
    /// Creates an empty binary with the specified capacity for the storage.
    ///
    /// # Panics
    /// The storage may panic if it has fixed capacity and the specified value does not match it.
    #[inline(always)]
    pub fn with_capacity(capacity: usize, root: L) -> Self {
        let mut storage = S::with_capacity(capacity);
        let root = storage.add(
            unsafe {
                // SAFETY: as above
                Node::root(root)
            }
        );
        Self {storage, root}
    }
    /// Returns a reference to the root node of the tree.
    #[inline(always)]
    #[allow(clippy::missing_const_for_fn)] // there cannot be constant trees just yet
    pub fn root(&self) -> NodeRef<'_, B, L, K, S> {
        unsafe {
            // SAFETY: binary trees cannot be created without a root
            NodeRef::new_raw_unchecked(self, self.root.clone())
        }
    }
}
impl<B, L, S> BinaryTree<B, L, usize, SparseStorage<Node<B, L, usize>, S>>
where S: ListStorage<Element = SparseStorageSlot<Node<B, L, usize>>> {
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
impl<B, L, K, S> Traversable for BinaryTree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    type Branch = B;
    type Leaf = L;
    type Cursor = K;

    fn advance_cursor<V>(
        &self,
        cursor: Self::Cursor,
        direction: VisitorDirection<V>,
    ) -> Result<Self::Cursor, CursorDirectionError> {
        let node = NodeRef::new_raw(self, cursor)
            .expect("the node specified by the cursor does not exist");
        match direction {
            VisitorDirection::Parent => {
                node.parent().ok_or(CursorDirectionError).map(|x| x.raw_key())
            },
            VisitorDirection::NextSibling => todo!(), // TODO
            VisitorDirection::Child(num) => match num {
                0 => node.left_child()
                    .ok_or(CursorDirectionError)
                    .map(|x| x.raw_key()),
                1 => node.right_child()
                    .ok_or(CursorDirectionError)
                    .map(|x| x.raw_key()),
                _ => Err(CursorDirectionError),
            },
            VisitorDirection::Stop(_) => Err(CursorDirectionError),
        }
    }
    #[inline(always)]
    fn cursor_to_root(&self) -> Self::Cursor {
        self.root.clone()
    }
    fn value_at(
        &self,
        cursor: Self::Cursor,
    ) -> NodeValue<&'_ Self::Branch, &'_ Self::Leaf> {
        let node_ref = NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        node_ref.value()
    }
    #[inline]
    #[track_caller]
    fn num_children_at(&self, cursor: Self::Cursor) -> usize {
        let node_ref = NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        if node_ref.is_full_branch() {
            2
        } else if node_ref.is_branch() {
            1
        } else {0}
    }
}
impl<B, L, K, S> TraversableMut for BinaryTree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    const CAN_REMOVE_INDIVIDUAL_CHILDREN: bool = true;
    #[inline(always)]
    fn value_mut_at(
        &mut self,
        cursor: Self::Cursor,
    ) -> NodeValue<&'_ mut Self::Branch, &'_ mut Self::Leaf> {
        self.storage.get_mut(&cursor)
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .value
            .as_mut()
            .into_value()
    }
    #[inline(always)]
    fn try_remove_leaf_with<F>(
        &mut self,
        cursor: Self::Cursor,
        f: F,
    ) -> Result<Self::Leaf, crate::TryRemoveLeafError>
    where F: FnOnce() -> Self::Leaf {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_leaf_with(f)
    }
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    fn try_remove_branch_with<F>(
        &mut self,
        cursor: Self::Cursor,
        f: F,
    ) -> Result<(Self::Branch, Self::Leaf, Option<Self::Leaf>), crate::TryRemoveBranchError>
    where F: FnOnce() -> Self::Leaf {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_branch_with(f)
    }
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    fn try_remove_children_with<F>(
        &mut self,
        cursor: Self::Cursor,
        f: F,
    ) -> Result<(Self::Branch, Self::Leaf, Option<Self::Leaf>), crate::TryRemoveChildrenError>
    where F: FnOnce() -> Self::Leaf {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_children_with(f)
    }
}
impl<B, L, K, S> Default for BinaryTree<B, L, K, S>
where
    L: Default,
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    #[inline(always)]
    fn default() -> Self {
        Self::new(L::default())
    }
}

/// A binary tree which uses a *sparse* `Vec` as backing storage.
///
/// The default `BinaryTree` type already uses this, so this is only provided for explicitness and consistency.
#[cfg(feature = "alloc")]
#[allow(unused_qualifications)]
pub type SparseVecBinaryTree<B, L = B> = BinaryTree<B, L, usize, SparseVec<Node<B, L, usize>>>;
/// A binary tree which uses a `Vec` as backing storage.
///
/// The default `BinaryTree` type uses `Vec` with sparse storage. Not using sparse storage is heavily discouraged, as the memory usage penalty is negligible. Still, this is provided for convenience.
#[cfg(feature = "alloc")]
#[allow(unused_qualifications)]
pub type VecBinaryTree<B, L = B> = BinaryTree<B, L, usize, alloc::vec::Vec<Node<B, L, usize>>>;

/*
/// A binary tree which uses a `LinkedList` as backing storage.
///
/// This is highly likely a bad idea.
#[cfg(feature = "linked_list_storage")]
pub type LinkedListBinaryTree<B, L> = BinaryTree<B, L, usize, alloc::collections::LinkedList<Node<B, L, usize>>>;
*/