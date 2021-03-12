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

use core::fmt::{self, Formatter, Debug, Display};

mod base;
mod impl_traversable;
mod node;
mod node_ref;
mod node_ref_mut;

use node::NodeData;
pub use node::Node;
pub use node_ref::{
    NodeRef,
    NodeChildrenIter,
    NodeChildKeysIter,
    NodeSiblingsIter,
    NodeSiblingKeysIter,
};
pub use node_ref_mut::NodeRefMut;
pub use base::FreeformTree;

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
