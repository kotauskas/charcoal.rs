//! Implements arena-allocated tree data structures and interfaces to work with them.
//!
//! # Overview
//! Charcoal implements various kinds of trees using a technique called ["arena-allocated trees"][arena tree blog post], described by Ben Lovy. The gist of it is that the trees use some sort of backing storage to store the elements, typically a [`Vec`] (or its variants, like [`SmallVec`] or [`ArrayVec`]), and instead of using pointers to link to children, indices into the storage are used instead. This significantly improves element insertion and removal performance as compared to `Rc`-based trees, and gives room for supporting configurations without a global memory allocator.
//!
//! # Storage
//! Charcoal uses [Granite] to handle arena-allocated storage. Several feature flags are used to enable various dependencies on various storage types via forwaring them to Granite.
//!
//! # Feature flags
//! - `std` (**enabled by default**) — enables the full standard library, disabling `no_std` for the crate. Currently, this only adds [`Error`] trait implementations for some types.
//! - `unwind_safety` (**enabled by default**) — **Must be enabled when using the unwinding panic implementation, otherwise using methods which accept closures is undefined behavior.** Requires `std`. Not a concern in `no_std` builds, since those do not have a panicking runtime by default.
//! - `alloc` (**enabled by default**) — adds `ListStorage` trait implementations for standard library containers, except for `LinkedList`, which is temporarily unsupported. *This does not require standard library support and will only panic at runtime in `no_std` environments without an allocator.*
//! - `smallvec` — forwarded to Granite, adds a `ListStorage` trait implementation for [`SmallVec`].
//! - `slab` — forwarded to Granite, adds a `Storage` trait implementation for [`Slab`].
//! - `slotmap` — forwarded to Granite, adds `Storage` trait implementations for [`SlotMap`], [`HopSlotMap`] and [`DenseSlotMap`].
//! - `union_optimizations` — forwarded to Granite, adds some layout optimizations by using untagged unions, decreasing memory usage in `SparseStorage`. **Requires a nightly compiler** (see [tracking issue for RFC 2514]) and thus is disabled by default.
//!
//! # Public dependencies
//! - `arrayvec` (**required**) — `^0.5`
//! - `granite` (**required**) — `^1.0`
//!     - `smallvec` (*optional*) — `^1.4`
//!     - `slab` (*optional*) — `^0.4`
//!     - `slotmap` (*optional*) — `^0.4`
//!
//! # Contributing
//! You can help by contributing to Charcoal in those aspects:
//! - **Algorithm optimizations** — Charcoal implements various ubiquitous algorithms for trees, and while those use a considerable amount of unsafe code, they still are never perfect and can be improved. If you find a way to improve an implementation of an algorithm in Charcoal, you're welcome to submit a PR implementing your improvement.
//! - **Testing, debugging and soundness auditing** — the development cycle of Charcoal prefers quality over quantity of releases. You can help with releasing new versions faster by contributing tests and reporting potential bugs and soundness holes — those should be very rare but it's very important that they are figured out and solved before being released in a new version of the crate.
//! - **Implementing more trees** — tree data structures come in various shapes and sizes. The code for the individual trees themselves strives to be consistent, so looking into any of the existing trees will be enough to implement your own. Charcoal aims to be the catch-all crate for all types of trees, so feel free to submit a direct PR to add your tree type instead of publishing your own Charcoal-based crate.
//!
//! [`Error`]: https://doc.rust-lang.org/std/error/trait.Error.html " "
//! [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html " "
//! [`VecDeque`]: https://doc.rust-lang.org/std/collections/struct.VecDeque.html " "
//! [`SmallVec`]: https://docs.rs/smallvec/*/smallvec/struct.SmallVec.html " "
//! [`ArrayVec`]: https://docs.rs/arrayvec/*/arrayvec/struct.ArrayVec.html " "
//! [`Slab`]: https://docs.rs/slab/*/slab/struct.Slab.html " "
//! [`SlotMap`]: https://docs.rs/slotmap/*/slotmap/struct.SlotMap.html " "
//! [`HopSlotMap`]: https://docs.rs/slotmap/*/slotmap/hop/struct.HopSlotMap.html " "
//! [`DenseSlotMap`]: https://docs.rs/slotmap/*/slotmap/dense/struct.DenseSlotMap.html " "
//! [Granite]: https://docs.rs/granite/*/granite/ " "
//! [tracking issue for RFC 2514]: https://github.com/rust-lang/rust/issues/55149 " "
//! [arena tree blog post]: https://dev.to/deciduously/no-more-tears-no-more-knots-arena-allocated-trees-in-rust-44k6 " "

#![warn(
    rust_2018_idioms,
    clippy::cargo,
    clippy::pedantic,
    clippy::nursery,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    // Broken, will display warnings even for undocumented items, including trait impls
    //missing_doc_code_examples,
    unused_qualifications,
    variant_size_differences,
    clippy::unwrap_used, // Only .expect() allowed
)]
#![deny(anonymous_parameters, bare_trait_objects)]
#![allow(
    clippy::use_self, // FIXME reenable when it gets fixed
    clippy::clippy::wildcard_imports, // Worst lint ever
    clippy::clippy::module_name_repetitions, // Annoying and stupid
    clippy::shadow_unrelated, // Countless false positives, very annoying
)]
#![cfg_attr(not(feature = "std"), no_std)]
// TODO reimplement LinkedList
//#![cfg_attr(feature = "linked_list_storage", feature(linked_list_cursors))]
#![cfg_attr(feature = "doc_cfg", feature(doc_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;

pub extern crate granite as storage;
#[doc(no_inline)]
pub use storage::{Storage, ListStorage, DefaultStorage};

#[cfg(feature = "binary_tree")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "binary_tree")))]
pub mod binary_tree;
#[cfg(feature = "binary_tree")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "binary_tree")))]
pub use binary_tree::BinaryTree;

#[cfg(feature = "octree")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "octree")))]
pub mod octree;
#[cfg(feature = "octree")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "octree")))]
pub use octree::Octree;

#[cfg(feature = "quadtree")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "quadtree")))]
pub mod quadtree;
#[cfg(feature = "quadtree")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "quadtree")))]
pub use quadtree::{Quadtree};

#[cfg(feature = "freeform_tree")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "freeform_tree")))]
pub mod freeform_tree;
#[cfg(feature = "freeform_tree")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "freeform_tree")))]
pub use freeform_tree::{FreeformTree};

pub mod traversal;
pub use traversal::{Visitor, VisitorMut, Traversable, TraversableMut};

/// A prelude for using Charcoal, containing the most used types in a renamed form for safe glob-importing.
pub mod prelude {
    #[cfg(feature = "binary_tree")]
    #[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "binary_tree")))]
    #[doc(no_inline)]
    pub use crate::binary_tree::{
        BinaryTree, NodeRef as BinaryTreeNodeRef, NodeRefMut as BinaryTreeNodeRefMut,
    };
    #[cfg(feature = "octree")]
    #[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "octree")))]
    #[doc(no_inline)]
    pub use crate::octree::{Octree, NodeRef as OctreeNodeRef, NodeRefMut as OctreeNodeRefMut};
    #[cfg(feature = "quadtree")]
    #[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "quadtree")))]
    #[doc(no_inline)]
    pub use crate::quadtree::{Quadtree, NodeRef as QuadtreeNodeRef, NodeRefMut as QuadtreeNodeRefMut};
    #[cfg(feature = "freeform_tree")]
    #[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "freeform_tree")))]
    #[doc(no_inline)]
    pub use crate::freeform_tree::{
        FreeformTree, NodeRef as FreeformTreeNodeRef, NodeRefMut as FreeformTreeNodeRefMut,
    };
}

pub(crate) mod util;

use core::fmt::{self, Formatter, Display, Debug};

/// The payload of a node of a tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeValue<B, L = B> {
    /// The payload of a branch node, i.e. a node with children. Those are also sometimes referred to as internal nodes or inodes.
    Branch(B),
    /// The payload of a leaf node, i.e. a node without any children.
    Leaf(L),
}
impl<B, L> NodeValue<B, L> {
    /// Converts from `&NodeValue<B, L>` to `NodeValue<&B, &L>`.
    pub const fn as_ref(&self) -> NodeValue<&B, &L> {
        match self {
            Self::Branch(x) => NodeValue::Branch(x),
            Self::Leaf(x) => NodeValue::Leaf(x),
        }
    }
    /// Converts from `&mut NodeValue<B, L>` to `NodeValue<&mut B, &mut L>`.
    pub fn as_mut(&mut self) -> NodeValue<&mut B, &mut L> {
        match self {
            Self::Branch(x) => NodeValue::Branch(x),
            Self::Leaf(x) => NodeValue::Leaf(x),
        }
    }
}
impl<T> NodeValue<T, T> {
    /// Extracts the value, discarding information about whether the node was a leaf or branch. *Available only if the leaf and branch payloads are the same type.*
    #[allow(clippy::missing_const_for_fn)]
    pub fn into_inner(self) -> T {
        match self {
            NodeValue::Branch(x) | NodeValue::Leaf(x) => x,
        }
    }
}
// FIXME a From conversion here might become possible at some point
impl<T> AsRef<T> for NodeValue<T, T> {
    fn as_ref(&self) -> &T {
        self.as_ref().into_inner()
    }
}
impl<T> AsRef<T> for NodeValue<&T, &T> {
    fn as_ref(&self) -> &T {
        self.into_inner()
    }
}
impl<T> AsMut<T> for NodeValue<T, T> {
    fn as_mut(&mut self) -> &mut T {
        self.as_mut().into_inner()
    }
}
impl<'a, T> AsMut<T> for NodeValue<&'a mut T, &'a mut T> {
    fn as_mut(&mut self) -> &mut T {
        match self {
            NodeValue::Branch(x) | NodeValue::Leaf(x) => x,
        }
    }
}

/// The error type returned by methods on trees which remove leaf nodes.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TryRemoveLeafError {
    /// The node was the root node, which cannot be removed.
    WasRootNode,
    /// The node was a branch node and thus would require recursion to remove.
    WasBranchNode,
    /// The tree does not support removing individual children or such support was manually disabled.
    CannotRemoveIndividualChildren,
}
impl Display for TryRemoveLeafError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            Self::WasRootNode => "cannot remove the root node of a tree",
            Self::WasBranchNode => "cannot remove branch nodes without recursion",
            Self::CannotRemoveIndividualChildren => {
                "removing individual children is not available for the tree"
            }
        })
    }
}
#[cfg(feature = "std")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "std")))]
impl std::error::Error for TryRemoveLeafError {}

/// The error type returned by methods on trees which remove branch nodes.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TryRemoveBranchError {
    /// The node was the root node, which cannot be removed.
    WasRootNode,
    /// The node a leaf node and thus should be removed with `try_remove_leaf_with` or a similar method.
    WasLeafNode,
    /// One of the node's children was a branch node, which would require recursion to remove. Contains the index of the offending node; if there were multiple, the smallest index is specified.
    HadBranchChild(u32),
    /// The tree does not support removing individual children or such support was manually disabled.
    CannotRemoveIndividualChildren,
}
impl Display for TryRemoveBranchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            Self::WasRootNode => "cannot remove the root node of a tree",
            Self::WasLeafNode => "expected a branch node, found leaf",
            Self::HadBranchChild(index) => {
                #[cfg(feature = "alloc")]
                {
                    return f.pad(&format!(
                        "\
node had a branch child (index {}), which cannot be removed without recursion",
                        index,
                    ));
                }
                #[cfg(not(feature = "alloc"))]
                {
                    "\
node had a branch child, which cannot be removed without recursion"
                }
            }
            Self::CannotRemoveIndividualChildren => {
                "removing individual children is not available for the tree"
            }
        })
    }
}
#[cfg(feature = "std")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "std")))]
impl std::error::Error for TryRemoveBranchError {}

/// The error type returned by methods on trees which remove children branch nodes.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TryRemoveChildrenError {
    /// The node a leaf node and thus cannot have children by definition.
    WasLeafNode,
    /// One of the node's children was a branch node, which would require recursion to remove. Contains the index of the offending node; if there were multiple, the smallest index is specified.
    HadBranchChild(u32),
}
impl Display for TryRemoveChildrenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            Self::WasLeafNode => "expected a branch node, found leaf",
            Self::HadBranchChild(index) => {
                #[cfg(feature = "alloc")]
                {
                    return f.pad(&format!(
                        "\
node had a branch child (index {}), which cannot be removed without recursion",
                        index,
                    ));
                }
                #[cfg(not(feature = "alloc"))]
                {
                    "\
node had a branch child, which cannot be removed without recursion"
                }
            }
        })
    }
}
#[cfg(feature = "std")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "std")))]
impl std::error::Error for TryRemoveChildrenError {}

/// The error type returned by methods on trees which convert leaf nodes into branch nodes, which occurs when the node which was attempted to be converted already is a branch node.
#[derive(Copy, Clone, Debug)]
pub struct MakeBranchError<L, P>
where
    P: IntoIterator<Item = L>,
{
    /// The packed children which were passed to the function and were deemed useless because the call failed, provided here so that they don't get dropped if they could instead be reused in the event of a failure.
    pub packed_children: P,
}
impl<L, P> Display for MakeBranchError<L, P>
where
    P: IntoIterator<Item = L>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad("the node already was a branch")
    }
}
#[cfg(feature = "std")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "std")))]
impl<L, P> std::error::Error for MakeBranchError<L, P>
where
    L: Debug,
    P: IntoIterator<Item = L> + Debug,
{
}
