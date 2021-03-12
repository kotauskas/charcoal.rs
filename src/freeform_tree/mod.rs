//! Freeform trees, ones which don't impose any restrictions on the number of child nodes that a branch node can have.
//!
//! Those can be used to build almost any other tree structure with ease, including some that are intentionally not implemented by Charcoal because they are just freeform trees with a specific type of key, such as the [trie].
//!
//! # Example
//! ```rust
//! use charcoal::freeform_tree::{FreeformTree, NodeRef};
//!
//! // Create the tree. The only thing we need for that is the data payload for the root node. The
//! // turbofish there is needed to state that we are using the default storage method instead of
//! // asking the compiler to infer it, which would be impossible.
//! let mut tree = FreeformTree::<_>::new(451);
//!
//! // Let's now try to access the structure of the tree and look around.
//! let root = tree.root();
//! // We have never added any nodes to the tree, so the root does not have any children, hence:
//! assert!(root.is_leaf());
//!
//! // Let's replace our reference to the root with a mutable one, to mutate the tree!
//! let mut root = tree.root_mut();
//! // First things first, we want to change our root's data payload:
//! *(root.value_mut().into_inner()) = 120;
//! // While we're at it, let's add some child nodes:
//! let my_numbers = [
//!     2014, 1987, 1983,
//! ];
//! root.make_branch(my_numbers.iter().copied()).unwrap();
//!
//! // Let's return to an immutable reference and look at our tree.
//! let root = NodeRef::from(root); // Conversion from a mutable to an immutable reference
//! assert_eq!(root.value().into_inner(), &120);
//! let children = {
//!     let mut children_ref_iter = root.children().unwrap();
//!     let mut get_val = |x| {
//!         // Type inference decided to abandon us here
//!         let x: NodeRef<'_, _, _, _> = children_ref_iter.next().unwrap();
//!         *x.value().into_inner()
//!     };
//!     [ get_val(0), get_val(1), get_val(2) ]
//! };
//! assert_eq!(children, my_numbers);
//! ```
//!
//! [trie]: https://en.wikipedia.org/wiki/Trie " "

use core::{
    fmt::{self, Formatter, Debug, Display},
    iter::Empty,
};
use crate::{
    storage::{Storage, ListStorage, DefaultStorage, SparseStorage, SparseStorageSlot},
    traversal::{
        Traversable,
        TraversableMut,
        VisitorDirection,
        CursorResult,
        CursorDirectionError,
    },
    NodeValue,
    TryRemoveBranchError,
    TryRemoveLeafError,
    TryRemoveChildrenError,
};

mod node;
mod node_ref;

use node::NodeData;
pub use node::Node;
pub use node_ref::{NodeRef, NodeRefMut, NodeSiblingsIter, NodeSiblingKeysIter};

/// A freeform tree.
///
/// See the [module-level documentation] for more.
///
/// [module-level documentation]: index.html " "
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FreeformTree<B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    storage: S,
    root: K,
}
impl<B, L, K, S> FreeformTree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a freeform tree with the specified value for the root node.
    ///
    /// # Example
    /// ```rust
    /// # use charcoal::FreeformTree;
    /// // The only way to create a tree...
    /// let tree = FreeformTree::<_>::new(87);
    /// // ...is to simply create the root leaf node and storage. The turbofish there is needed to
    /// // state that we are using the default storage method instead of asking the compiler to
    /// // infer it, which would be impossible.
    ///
    /// // No other nodes have been created yet:
    /// assert!(tree.root().is_leaf());
    /// ```
    #[inline(always)]
    pub fn new(root: L) -> Self {
        let mut storage = S::new();
        let root = storage.add(unsafe {
            // SAFETY: there isn't a root there yet
            Node::root(root)
        });
        Self { storage, root }
    }
    /// Creates a freeform tree with the specified capacity for the storage.
    ///
    /// # Panics
    /// The storage may panic if it has fixed capacity and the specified value does not match it.
    ///
    /// # Example
    /// ```rust
    /// # use charcoal::FreeformTree;
    /// // Let's create a tree, but with some preallocated space for more nodes:
    /// let mut tree = FreeformTree::<_>::with_capacity(5, "Variable Names");
    /// // The turbofish there is needed to state that we are using the default storage method
    /// // instead of asking the compiler to infer it, which would be impossible.
    ///
    /// // Capacity does not affect the actual nodes:
    /// assert!(tree.root().is_leaf());
    ///
    /// // Not until we create them ourselves:
    /// tree.root_mut().make_branch([
    ///     "Foo", "Bar", "Baz", "Quux",
    /// ].iter().copied());
    ///
    /// // If the default storage is backed by a dynamic memory allocation,
    /// // at most one has happened to this point.
    /// ```
    #[inline(always)]
    pub fn with_capacity(capacity: usize, root: L) -> Self {
        let mut storage = S::with_capacity(capacity);
        let root = storage.add(unsafe {
            // SAFETY: as above
            Node::root(root)
        });
        Self { storage, root }
    }

    /// Returns a reference to the root node of the tree.
    ///
    /// # Example
    /// ```rust
    /// # use charcoal::FreeformTree;
    /// // A tree always has a root node:
    /// let tree = FreeformTree::<_>::new("Root");
    ///
    /// assert_eq!(
    ///     // The into_inner() call extracts data from a NodeValue, which is used to generalize
    ///     // tres to both work with same and different types for payloads of leaf and branch
    ///     // nodes.
    ///     *tree.root().value().into_inner(),
    ///     "Root",
    /// );
    /// ```
    #[inline(always)]
    #[allow(clippy::missing_const_for_fn)] // there cannot be constant trees just yet
    pub fn root(&self) -> NodeRef<'_, B, L, K, S> {
        unsafe {
            // SAFETY: binary trees cannot be created without a root
            NodeRef::new_raw_unchecked(self, self.root.clone())
        }
    }
    /// Returns a *mutable* reference to the root node of the tree, allowing modifications to the entire tree.
    ///
    /// # Example
    /// ```rust
    /// # use charcoal::FreeformTree;
    /// // A tree always has a root node:
    /// let mut tree = FreeformTree::<_>::new("Root");
    ///
    /// let mut root_mut = tree.root_mut();
    /// // The into_inner() call extracts data from a NodeValue, which is used to generalize trees
    /// // to both work with same and different types for payloads of leaf and branch nodes.
    /// *(root_mut.value_mut().into_inner()) = "The Source of the Beer";
    /// ```
    #[inline(always)]
    pub fn root_mut(&mut self) -> NodeRefMut<'_, B, L, K, S> {
        unsafe {
            // SAFETY: as above
            NodeRefMut::new_raw_unchecked(self, self.root.clone())
        }
    }
}
impl<B, L, S> FreeformTree<B, L, usize, SparseStorage<Node<B, L, usize>, S>>
where
    S: ListStorage<Element = SparseStorageSlot<Node<B, L, usize>>>,
{
    /// Removes all holes from the sparse storage.
    ///
    /// Under the hood, this uses `defragment_and_fix`. It's not possible to defragment without fixing the indicies, as that might cause undefined behavior.
    ///
    /// # Example
    /// ```rust
    /// use charcoal::freeform_tree::SparseVecFreeformTree;
    ///
    /// // Create a tree which explicitly uses sparse storage:
    /// let mut tree = SparseVecFreeformTree::new(0);
    /// // This is already the default, but for the sake of this example we'll stay explicit.
    ///
    /// // Add some elements for the holes to appear:
    /// tree.root_mut().make_branch([
    ///     1, 2, 3, 4, 5,
    /// ].iter().copied()).unwrap(); // You can replace this with proper error handling
    /// tree
    ///     .root_mut()
    ///     .first_child_mut()
    ///     .unwrap() // This too
    ///     .make_branch([
    ///         6, 7, 8, 9, 10,
    ///     ].iter().copied())
    ///     .unwrap(); // And this
    ///
    /// tree
    ///     .root_mut()
    ///     .first_child_mut()
    ///     .unwrap() // Same as above
    ///     .try_remove_children(drop)
    ///     .unwrap(); // Same here
    ///
    /// // We ended up creating 5 holes:
    /// assert_eq!(tree.num_holes(), 5);
    /// // Let's patch them:
    /// tree.defragment();
    /// // Now there are none:
    /// assert_eq!(tree.num_holes(), 0);
    /// ```
    #[inline(always)]
    pub fn defragment(&mut self) {
        self.storage.defragment_and_fix()
    }
    /// Returns the number of holes in the storage. This operation returns immediately instead of looping through the entire storage, since the sparse storage automatically tracks the number of holes it creates and destroys.
    ///
    /// # Example
    /// See the example in [`defragment`].
    ///
    /// [`defragment`]: #method.defragment " "
    #[inline(always)]
    pub fn num_holes(&self) -> usize {
        self.storage.num_holes()
    }
    /// Returns `true` if there are no holes in the storage, `false` otherwise. This operation returns immediately instead of looping through the entire storage, since the sparse storage automatically tracks the number of holes it creates and destroys.
    ///
    /// # Example
    /// See the example in [`defragment`].
    ///
    /// [`defragment`]: #method.defragment " "
    #[inline(always)]
    pub fn is_dense(&self) -> bool {
        self.storage.is_dense()
    }
}
impl<B, L, K, S> Traversable for FreeformTree<B, L, K, S>
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
            VisitorDirection::Parent => node.parent().map(NodeRef::into_raw_key).ok_or(error),
            VisitorDirection::NextSibling => {
                node.next_sibling().map(NodeRef::into_raw_key).ok_or(error)
            }
            VisitorDirection::Child(num) => node
                .children_keys()
                .and_then(|mut x| x.nth(num as usize))
                .ok_or(error),
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
        node_ref.children_keys().map_or(0, Iterator::count)
    }
    #[inline]
    #[track_caller]
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor> {
        NodeRef::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .children_keys()
            .and_then(|mut x| x.nth(child_num as usize))
    }
}
impl<B, L, K, S> TraversableMut for FreeformTree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    const CAN_REMOVE_INDIVIDUAL_CHILDREN: bool = true;
    type PackedChildren = Empty<L>;

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
    fn try_remove_leaf<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
    ) -> Result<Self::Leaf, TryRemoveLeafError> {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_leaf_with(branch_to_leaf)
    }
    #[inline(always)]
    fn try_remove_branch_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
        collector: C,
    ) -> Result<Self::Branch, TryRemoveBranchError> {
        NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor))
            .try_remove_branch_with(branch_to_leaf, collector)
    }
    #[inline]
    #[track_caller]
    fn try_remove_children_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
        collector: C,
    ) -> Result<(), TryRemoveChildrenError> {
        let mut node_ref = NodeRefMut::new_raw(self, cursor.clone())
            .unwrap_or_else(|| panic!("invalid cursor: {:?}", cursor));
        node_ref.try_remove_children_with(branch_to_leaf, collector)
    }
}

/// The error type produced by [`try_push_back`] and [`try_push_front`], indicating that the node was a leaf node before.
///
/// The same operation could be retried with [`push_back_with`]/[`push_front_with`], or [`push_back`]/[`push_front`] if the same type is used for leaf node and branch node payloads.
///
/// [`try_push_back`]: struct.NodeRefMut.html#method.try_push_back " "
/// [`try_push_front`]: struct.NodeRefMut.html#method.try_push_front " "
/// [`push_back_with`]: struct.NodeRefMut.html#method.push_back_with " "
/// [`push_front_with`]: struct.NodeRefMut.html#method.push_front_with " "
/// [`push_back`]: struct.NodeRefMut.html#method.push_back " "
/// [`push_front`]: struct.NodeRefMut.html#method.push_front " "
#[derive(Copy, Clone, Debug, Default, Hash)]
pub struct TryPushError<T> {
    /// The value of the child node which was attempted to be added, returned back to the caller to avoid dropping it.
    pub child_payload: T,
}
impl<T> Display for TryPushError<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad("try_push_back or try_push_front was attempted at a leaf node")
    }
}

/// A freeform tree which uses a *sparse* `Vec` as backing storage.
///
/// The default `FreeformTree` type already uses this, so this is only provided for explicitness and consistency.
#[cfg(feature = "alloc")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "alloc")))]
#[allow(unused_qualifications)]
pub type SparseVecFreeformTree<B, L = B> =
    FreeformTree<B, L, usize, crate::storage::SparseVec<Node<B, L, usize>>>;
/// A freeform tree which uses a `Vec` as backing storage.
///
/// The default `FreeformTree` type uses `Vec` with sparse storage. Not using sparse storage is heavily discouraged, as the memory usage penalty is negligible. Still, this is provided for convenience.
#[cfg(feature = "alloc")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "alloc")))]
#[allow(unused_qualifications)]
pub type VecFreeformTree<B, L = B> = FreeformTree<B, L, usize, alloc::vec::Vec<Node<B, L, usize>>>;
