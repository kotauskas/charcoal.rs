//! Trees which allow nodes to have either zero children or exactly **8**, most often used to partition a 3D space by recursively subdividing it into eight octants.
//!
//! The [Wikipedia article] on octrees covers their use cases and specifics in more detail.
//!
//! # Example
//! ```rust
//! use charcoal::octree::{Octree, NodeRef};
//!
//! // Create the tree. The only thing we need for that is the data payload for the root node. The
//! // turbofish there is needed to state that we are using the default storage method instead of
//! // asking the compiler to infer it, which would be impossible.
//! let mut tree = Octree::<_>::new(451);
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
//!     // Random numbers are not what you'd typically see in an octree, but for the sake of this
//!     // example we can use absolutely any kind of data. Bonus points for finding hidden meaning.
//!     2010, 2014, 1987, 1983, 1993, 2023, 621, 926,
//! ];
//! root.make_branch(my_numbers).unwrap();
//!
//! // Let's return to an immutable reference and look at our tree.
//! let root = NodeRef::from(root); // Conversion from a mutable to an immutable reference
//! assert_eq!(root.value().into_inner(), &120);
//! let children = {
//!     let children_refs = root.children().unwrap();
//!     let get_val = |x| {
//!         // Type inference decided to abandon us here
//!         let x: NodeRef<'_, _, _, _> = children_refs[x];
//!         *x.value().into_inner()
//!     };
//!     [
//!         get_val(0), get_val(1), get_val(2), get_val(3),
//!         get_val(4), get_val(5), get_val(6), get_val(7),
//!     ]
//! };
//! assert_eq!(children, my_numbers);
//! ```
//!
//! [Wikipedia article]: https://en.wikipedia.org/wiki/Octree " "

use core::{
    fmt::Debug,
    iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator},
    borrow::{Borrow, BorrowMut},
};
use arrayvec::{ArrayVec, IntoIter as ArrayVecIntoIter};

mod base;
mod impl_traversable;
mod node;
mod node_ref;
mod node_ref_mut;

use node::NodeData;
pub use node::Node;
pub use node_ref::NodeRef;
pub use node_ref_mut::NodeRefMut;
pub use base::Octree;

/// Packed leaf children nodes of an octree's branch node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PackedChildren<T>(pub [T; 8]);
impl<T> PackedChildren<T> {
    /// Returns the packed children as an array.
    #[allow(clippy::missing_const_for_fn)] // cannot drop at compile time smh
    pub fn into_inner(self) -> [T; 8] {
        self.0
    }
}
impl<T> Borrow<[T]> for PackedChildren<T> {
    fn borrow(&self) -> &[T] {
        &self.0
    }
}
impl<T> BorrowMut<[T]> for PackedChildren<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut self.0
    }
}
impl<T> IntoIterator for PackedChildren<T> {
    type Item = T;
    type IntoIter = PackedChildrenIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.into()
    }
}
impl<T> From<[T; 8]> for PackedChildren<T> {
    fn from(op: [T; 8]) -> Self {
        Self(op)
    }
}

/// An owned iterator over the elements of `PackedChildren`.
#[derive(Clone, Debug)]
pub struct PackedChildrenIter<T>(ArrayVecIntoIter<[T; 8]>);
impl<T> From<PackedChildren<T>> for PackedChildrenIter<T> {
    fn from(op: PackedChildren<T>) -> Self {
        Self(ArrayVec::from(op.0).into_iter())
    }
}
impl<T> Iterator for PackedChildrenIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<T> DoubleEndedIterator for PackedChildrenIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
impl<T> ExactSizeIterator for PackedChildrenIter<T> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl<T> FusedIterator for PackedChildrenIter<T> {}

/// An octree which uses a *sparse* `Vec` as backing storage.
///
/// The default `Octree` type already uses this, so this is only provided for explicitness and consistency.
#[cfg(feature = "alloc")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "alloc")))]
#[allow(unused_qualifications)]
pub type SparseVecOctree<B, L = B> =
    Octree<B, L, usize, crate::storage::SparseVec<Node<B, L, usize>>>;
/// An octree which uses a `Vec` as backing storage.
///
/// The default `Octree` type uses `Vec` with sparse storage. Not using sparse storage is heavily discouraged, as the memory usage penalty is negligible. Still, this is provided for convenience.
#[cfg(feature = "alloc")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "alloc")))]
#[allow(unused_qualifications)]
pub type VecOctree<B, L = B> = Octree<B, L, usize, alloc::vec::Vec<Node<B, L, usize>>>;
