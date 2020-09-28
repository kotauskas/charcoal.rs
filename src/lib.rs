//! Implements arena-allocated tree data structures and interfaces to work with them.
//!
//! ------------------------
//!
//! # Not yet release ready
//! Currently, the crate is in an unfinished state and is not ready to be uploaded to Crates.io. Here's an overview of everything that needs to be finished in order to consider the crate production-ready:
//! - More trees!
//! - Built-in algorithms
//! - More comprehensive documentation, examples
//! - Tests, examples, possibly also benchmarks with `criterion.rs`
//!
//! ------------------------
//!
//! # Overview
//! Charcoal implements various kinds of trees using a technique called ["arena-allocated trees"][arena tree blog post], described by Ben Lovy. The gist of it is that the trees use some sort of backing storage to store the elements, typically a [`Vec`] (or its variants, like [`SmallVec`] or [`ArrayVec`]), and instead of using pointers to link to children, indices into the storage are used instead. This significantly improves element insertion and removal performance as compared to `Rc`-based trees, and gives room for supporting configurations without a global memory allocator.
//!
//! # Storage
//! The trait used for defining the "arena" type used is `Storage`. Implementing it directly isn't the only way to get your type to be supported by tree types — `ListStorage` is a trait which allows you to define an arena storage in terms of a list-like collection.
//!
//! Several types from both the standard library and external crates already implement `Storage` and `ListStorage` out of the box:
//! - [`Vec`], [`SmallVec`] and [`ArrayVec`] — `ListStorage`
//! - [`VecDeque`] — `ListStorage`, does not use `VecDeque` semantics and is simply provided for convenience
//! - [`SlotMap`], [`HopSlotMap`] and [`DenseSlotMap`] — `Storage`
//!
//! You can opt out of one or multiple of those implementations using feature flags as described by the *Feature flags* section.
//!
//! ## Sparse storage
//! By default, all trees use a technique called "sparse storage" to significantly speed up element removal. (As a side effect, element insertion is also sometimes faster.) It has two side effects:
//! - Element size increases because of the additional `Slot<T>` layer used to implement sparse storage. Due to alignment, node size gets increased by a whole `usize`.
//! - As elements get removed, they will leave holes behind themselves. Those are usually cleaned up automatically as new elements are inserted, **but if you need to clean up all at once, you can use `defragment`.**
//!
//! # Feature flags
//! - `std` (**enabled by default**) - enables the full standard library, disabling `no_std` for the crate. Currently, this only adds [`Error`] trait implementations for some types.
//! - `alloc` (**enabled by default**) — adds `ListStorage` trait implementations for standard library containers, except for `LinkedList`, which is temporarily unsupported. *This does not require standard library support and will only panic at runtime in `no_std` environments without an allocator.*
//! - `smallvec_storage` (**enabled by default**) — adds a `ListStorage` trait implementation for [`SmallVec`].
//! - `arrayvec_storage` (**enabled by default**) — adds a `ListStorage` trait implementation for [`ArrayVec`].
//! - `slotmap_storage` (**enabled by default**) — adds `Storage` trait implementations for [`SlotMap`], [`HopSlotMap`] and [`DenseSlotMap`].
//! - `union_optimizations` — adds some layout optimizations by using untagged unions, decreasing memory usage in `SparseStorage`. **Requires a nightly compiler** (see [tracking issue for RFC 2514]) and thus is disabled by default.
//!
//! [`Error`]: https://doc.rust-lang.org/std/error/trait.Error.html " "
//! [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html " "
//! [`VecDeque`]: https://doc.rust-lang.org/std/collections/struct.VecDeque.html " "
//! [`SmallVec`]: https://docs.rs/smallvec/*/smallvec/struct.SmallVec.html " "
//! [`ArrayVec`]: https://docs.rs/arrayvec/*/arrayvec/struct.ArrayVec.html " "
//! [`SlotMap`]: https://docs.rs/slotmap/*/slotmap/struct.SlotMap.html " "
//! [`HopSlotMap`]: https://docs.rs/slotmap/*/slotmap/hop/struct.HopSlotMap.html " "
//! [`DenseSlotMap`]: https://docs.rs/slotmap/*/slotmap/dense/struct.DenseSlotMap.html " "
//! [tracking issue for RFC 2514]: https://github.com/rust-lang/rust/issues/55149 " "
//! [arena tree blog post]: https://dev.to/deciduously/no-more-tears-no-more-knots-arena-allocated-trees-in-rust-44k6 " "

#![warn(
    rust_2018_idioms,
    clippy::cargo,
    clippy::nursery,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unused_qualifications,
    variant_size_differences,
    clippy::cast_lossless,
    clippy::await_holding_lock,
    clippy::checked_conversions,
    clippy::copy_iterator,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_iter_loop,
    clippy::explicit_into_iter_loop,
    clippy::filter_map,
    clippy::filter_map_next,
    clippy::find_map,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::fn_params_excessive_bools,
    clippy::implicit_hasher,
    clippy::implicit_saturating_sub,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::items_after_statements,
    clippy::large_stack_arrays,
    clippy::let_unit_value,
    clippy::macro_use_imports,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    // sick of this stupid lint, disabling
    // clippy::module_name_repetitions,
    clippy::mut_mut,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_if_let_else,
    clippy::option_option,
    clippy::pub_enum_variant_names,
    clippy::range_plus_one,
    clippy::range_minus_one,
    clippy::redundant_closure_for_method_calls,
    clippy::same_functions_in_if_condition,
    // also sick of this one, gives too much false positives inherent to its design
    // clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::string_add_assign,
    clippy::too_many_lines,
    clippy::type_repetition_in_bounds,
    clippy::trivially_copy_pass_by_ref,
    clippy::unicode_not_nfc,
    clippy::unnested_or_patterns,
    clippy::unsafe_derive_deserialize,
    clippy::unused_self,
    clippy::used_underscore_binding,
)]
#![deny(anonymous_parameters, bare_trait_objects)]
#![allow(clippy::use_self)] // FIXME reenable when it gets fixed
#![cfg_attr(not(feature = "std"), no_std)]
// TODO reimplement LinkedList
//#![cfg_attr(feature = "linked_list_storage", feature(linked_list_cursors))]
#![cfg_attr(feature = "union_optimizations", feature(untagged_unions))]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod storage;
pub use storage::{Storage, ListStorage, DefaultStorage};

pub mod binary_tree;
pub use binary_tree::BinaryTree;

pub mod traversal;
pub use traversal::{Visitor, VisitorMut, Traversable, TraversableMut};

/// A prelude for using Charcoal, containing the most used types in a renamed form for safe glob-importing.
pub mod prelude {
    pub use crate::storage::ListStorage as TreeStorage;
    pub use crate::storage::SparseStorage as SparseTreeStorage;
    pub use crate::storage::DefaultStorage as DefaultTreeStorage;
    pub use crate::binary_tree::BinaryTree;
}

/*#[cfg(any(feature = "smallvec_storage", feature = "arrayvec_storage"))]
const INLINE_STACK_SIZE: usize = 128;

#[cfg(feature = "smallvec_storage")]
pub(crate) type Stack<T> = smallvec::SmallVec<[T; INLINE_STACK_SIZE]>;
#[cfg(all(feature = "alloc", not(feature = "smallvec_storage")))]
pub(crate) type Stack<T> = alloc::vec::Vec<T>;
#[cfg(all(
    feature = "arrayvec_storage",
    not(feature = "smallvec_storage"),
    not(feature = "alloc"),
))]
pub(crate) type Stack<T> = arrayvec::ArrayVec<[T; INLINE_STACK_SIZE]>;*/

use core::fmt::{self, Formatter, Display};

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
    #[inline]
    pub const fn as_ref(&self) -> NodeValue<&B, &L> {
        match self {
            Self::Branch(x) => NodeValue::Branch(x),
            Self::Leaf(x) => NodeValue::Leaf(x),
        }
    }
    /// Converts from `&mut NodeValue<B, L>` to `NodeValue<&mut B, &mut L>`.
    #[inline]
    pub fn as_mut(&mut self) -> NodeValue<&mut B, &mut L> {
        match self {
            Self::Branch(x) => NodeValue::Branch(x),
            Self::Leaf(x) => NodeValue::Leaf(x),
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
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WasRootNode => "cannot remove the root node of a tree",
            Self::WasBranchNode => "cannot remove branch nodes without recursion",
            Self::CannotRemoveIndividualChildren => {
                "removing individual children is not available for the tree"
            }
        })
    }
}
#[cfg(feature = "std")]
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
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WasRootNode => "cannot remove the root node of a tree",
            Self::WasLeafNode => "expected a branch node, found leaf",
            Self::HadBranchChild(index) => {
                #[cfg(feature = "alloc")]
                {
                    return f.write_str(&format!(
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
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WasLeafNode => "expected a branch node, found leaf",
            Self::HadBranchChild(index) => {
                #[cfg(feature = "alloc")]
                {
                    return f.write_str(&format!(
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
impl std::error::Error for TryRemoveChildrenError {}
