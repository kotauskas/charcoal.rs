# Sapling
[![Crates.io](https://img.shields.io/crates/v/sapling)](https://crates.io/crates/sapling "Sapling on Crates.io")
[![Docs.rs](https://img.shields.io/badge/documentation-docs.rs-informational)](https://docs.rs/sapling "Sapling on Docs.rs")
[![Build Status](https://github.com/kotauskas/sapling.rs/workflows/Build/badge.svg)](https://github.com/kotauskas/sapling.rs/actions "GitHub Actions page for Sapling")

Implements arena-allocated tree data structures and interfaces to work with them.

------------------------

## Not yet release ready
Currently, the crate is in an unfinished state and is not ready to be uploaded to Crates.io. Here's an overview of everything that needs to be finished in order to consider the crate production-ready:
- Built-in algorithms
- More comprehensive documentation, examples
- Tests, examples, possibly also benchmarks with `bencher`

------------------------

## Overview
Sapling implements various kinds of trees using a technique called ["arena-allocated trees"][arena tree blog post], described by Ben Lovy. The gist of it is that the trees use some sort of backing storage to store the elements, typically a [`Vec`] (or its variants, like [`SmallVec`] or [`ArrayVec`]), and instead of using pointers to link to children, indices into the storage are used instead. This significantly improves element insertion and removal performance as compared to `Rc`-based trees, and gives room for supporting configurations without a global memory allocator.

## Storage
The trait used for defining the "arena" type used is `Storage`. Implementing it directly isn't the only way to get your type to be supported by tree types — `ListStorage` is a trait which allows you to define an arena storage in terms of a list-like collection.

Several types from both the standard library and external crates already implement `Storage` and `ListStorage` out of the box:
- [`Vec`], [`SmallVec`] and [`ArrayVec`] — `ListStorage`
- [`VecDeque`] — `ListStorage`, does not use `VecDeque` semantics and is simply provided for convenience
- [`SlotMap`], [`HopSlotMap`] and [`DenseSlotMap`] — `Storage`

You can opt out of one or multiple of those implementations using feature flags as described by the *Feature flags* section.

### Sparse storage
By default, all trees use a technique called "sparse storage" to significantly speed up element removal. (As a side effect, element insertion is also sometimes faster.) It has two side effects:
- Element size increases because of the additional `Slot<T>` layer used to implement sparse storage. Due to alignment, node size gets increased by a whole `usize`.
- As elements get removed, they will leave holes behind themselves. Those are usually cleaned up automatically as new elements are inserted, **but if you need to clean up all at once, you can use `defragment`.**

## Feature flags
- `std` (**enabled by default**) - enables the full standard library, disabling `no_std` for the crate. Currently, this only adds [`Error`] trait implementations for some types.
- `alloc` (**enabled by default**) — adds `ListStorage` trait implementations for standard library containers, except for `LinkedList`, which is temporarily unsupported. *This does not require standard library support and will only panic at runtime in `no_std` environments without an allocator.*
- `smallvec_storage` (**enabled by default**) — adds a `ListStorage` trait implementation for [`SmallVec`].
- `arrayvec_storage` (**enabled by default**) — adds a `ListStorage` trait implementation for [`ArrayVec`].
- `slotmap_storage` (**enabled by default**) — adds `Storage` trait implementations for [`SlotMap`], [`HopSlotMap`] and [`DenseSlotMap`].
- `union_optimizations` — adds some layout optimizations by using untagged unions, decreasing memory usage in `SparseStorage`. **Requires a nightly compiler** (see [tracking issue for RFC 2514]) and thus is disabled by default.
