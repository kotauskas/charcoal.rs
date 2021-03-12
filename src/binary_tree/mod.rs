//! Trees which allow at most two children for their nodes.
//!
//! The [Wikipedia article] on binary trees covers their use cases and specifics in more detail.
//!
//! Both *full* binary trees and non-full ones are supported. The former ones allow strictly either zero or two children, the latter ones also allow one child to exist without the other one. If there is only one, it's always treated as the left one, and removing the left child for a full branch will shift the right child into the position of the left one (implemented as a simple and very inexpensive key modification and does not actually move the elements themselves around).
//!
//! # Example
//! ```rust
//! use charcoal::binary_tree::{BinaryTree, NodeRef};
//!
//! // Create the tree. The only thing we need for that is the data payload for the root node. The
//! // turbofish there is needed to state that we are using the default storage method instead of
//! // asking the compiler to infer it, which would be impossible.
//! let mut tree = BinaryTree::<_>::new("Welcome".to_string());
//!
//! // Let's now try to access the structure of the tree and look around.
//! let root = tree.root();
//! // We have never added any nodes to the tree, so the root does not have any children, hence:
//! assert!(root.is_leaf());
//!
//! // Let's replace our reference to the root with a mutable one, to mutate the tree!
//! let mut root = tree.root_mut();
//! // First things first, we want to change our root's data payload:
//! *(root.value_mut().into_inner()) = "Hello".to_string();
//! // While we're at it, let's add some child nodes:
//! root.make_branch("World".to_string(), Some( "Rust".to_string() )).unwrap();
//!
//! // Let's return to an immutable reference and look at our tree.
//! let root = NodeRef::from(root); // Conversion from a mutable to an immutable reference
//! assert_eq!(root.value().into_inner(), "Hello");
//! let (left_child, right_child) = root.children().unwrap();
//! assert_eq!(left_child.value().into_inner(), "World");
//! assert_eq!(right_child.value().into_inner(), "Rust");
//! ```
//!
//! [Wikipedia article]: https://en.wikipedia.org/wiki/Binary_tree " "

use core::fmt::{self, Formatter, Debug, Display};

mod base;
mod impl_traversable;
mod node;
mod node_ref;
mod node_ref_mut;

use node::NodeData;
pub use node::Node;
pub use node_ref::NodeRef;
pub use node_ref_mut::{NodeRefMut};
pub use base::BinaryTree;

/// The error type returned by [`NodeRefMut::make_full_branch`].
///
/// [`NodeRefMut::make_full_branch`]: struct.NodeRefMut.html#method.make_full_branch " "
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MakeFullBranchError<L> {
    /// The node was a leaf node, not a partial branch. You can use [`make_branch`]/[`make_branch_with`] to add both children at once instead.
    ///
    /// [`make_branch`]: struct.NodeRefMut.html#method.make_branch " "
    /// [`make_branch_with`]: struct.NodeRefMut.html#method.make_branch_with " "
    WasLeafNode {
        /// The provided right child to add, which was deemed useless when the operation failed and is returned to the caller to avoid dropping it.
        right_child: L,
    },
    /// The node already was a full branch.
    WasFullBranch {
        /// The provided right child to add, which was deemed useless when the operation failed and is returned to the caller to avoid dropping it.
        right_child: L,
    },
}
impl<L> MakeFullBranchError<L> {
    /// Extracts the provided right child to add, which was deemed useless when the operation failed.
    #[allow(clippy::missing_const_for_fn)] // Clippy has no idea what a destructor is
    pub fn right_child(self) -> L {
        match self {
            Self::WasLeafNode { right_child } | Self::WasFullBranch { right_child } => right_child,
        }
    }
}
impl<L> Display for MakeFullBranchError<L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            Self::WasLeafNode { .. } => "the node was a leaf, not a partial branch",
            Self::WasFullBranch { .. } => "the node already was a full branch",
        })
    }
}
#[cfg(feature = "std")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "std")))]
impl<L: Debug> std::error::Error for MakeFullBranchError<L> {}

/// A binary tree which uses a *sparse* `Vec` as backing storage.
///
/// The default `BinaryTree` type already uses this, so this is only provided for explicitness and consistency.
#[cfg(feature = "alloc")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "alloc")))]
#[allow(unused_qualifications)]
pub type SparseVecBinaryTree<B, L = B> =
    BinaryTree<B, L, usize, crate::storage::SparseVec<Node<B, L, usize>>>;
/// A binary tree which uses a `Vec` as backing storage.
///
/// The default `BinaryTree` type uses `Vec` with sparse storage. Not using sparse storage is heavily discouraged, as the memory usage penalty is negligible. Still, this is provided for convenience.
#[cfg(feature = "alloc")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "alloc")))]
#[allow(unused_qualifications)]
pub type VecBinaryTree<B, L = B> = BinaryTree<B, L, usize, alloc::vec::Vec<Node<B, L, usize>>>;

/*
/// A binary tree which uses a `LinkedList` as backing storage.
///
/// This is highly likely a bad idea.
#[cfg(feature = "linked_list_storage")]
pub type LinkedListBinaryTree<B, L> = BinaryTree<B, L, usize, alloc::collections::LinkedList<Node<B, L, usize>>>;
*/
