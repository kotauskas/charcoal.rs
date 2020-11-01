# Charcoal
[![Crates.io](https://img.shields.io/crates/v/charcoal)](https://crates.io/crates/charcoal "Charcoal on Crates.io")
[![Docs.rs](https://img.shields.io/badge/documentation-docs.rs-informational)](https://docs.rs/charcoal "Charcoal on Docs.rs")
[![Checks and tests](https://github.com/kotauskas/charcoal.rs/workflows/Checks%20and%20tests/badge.svg)](https://github.com/kotauskas/charcoal.rs/actions "GitHub Actions page for Charcoal")
[![Minimal Supported Rust Version](https://img.shields.io/badge/msrv-1.46-orange)](https://blog.rust-lang.org/2020/08/27/Rust-1.46.0.html "Rust 1.46 release notes")

Implements arena-allocated tree data structures and interfaces to work with them.

------------------------

## Not yet release ready
Currently, the crate is in an unfinished state and is not ready to be uploaded to Crates.io. Here's an overview of everything that needs to be finished in order to consider the crate production-ready:
- More trees!
- Built-in algorithms
- More comprehensive documentation, examples
- Tests, examples, possibly also benchmarks with `criterion.rs`

------------------------

## Overview
Charcoal implements various kinds of trees using a technique called ["arena-allocated trees"][arena tree blog post], described by Ben Lovy. The gist of it is that the trees use some sort of backing storage to store the elements, typically a [`Vec`] (or its variants, like [`SmallVec`] or [`ArrayVec`]), and instead of using pointers to link to children, indices into the storage are used instead. This significantly improves element insertion and removal performance as compared to `Rc`-based trees, and gives room for supporting configurations without a global memory allocator.

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
- `smallvec` (**enabled by default**) — adds a `ListStorage` trait implementation for [`SmallVec`].
- `slotmap` (**enabled by default**) — adds `Storage` trait implementations for [`SlotMap`], [`HopSlotMap`] and [`DenseSlotMap`].
- `union_optimizations` — adds some layout optimizations by using untagged unions, decreasing memory usage in `SparseStorage`. **Requires a nightly compiler** (see [tracking issue for RFC 2514]) and thus is disabled by default.

## Public dependencies
- `arrayvec` (**required**) — `^0.5`
- `smallvec` (*optional*) — `^1.4`
- `slotmap` (*optional*) — `^0.4`

## Contributing
You can help by contributing to Charcoal in those aspects:
- **Algorithm optimizations** — Charcoal implements various ubiquitous algorithms for trees, and while those use a considerable amount of unsafe code, they still are never perfect and can be improved. If you find a way to improve an implementation of an algorithm in Charcoal, you're welcome to submit a PR implementing your improvement.
- **Testing, debugging and soundness auditing** — the development cycle of Charcoal prefers quality over quantity of releases. You can help with releasing new versions faster by contributing tests and reporting potential bugs and soundness holes — those should be very rare but it's very important that they are figured out and solved before being released in a new version of the crate.
- **Implementing more trees** — tree data structures come in various shapes and sizes. You can use the Tree Implementation Checklist (see `tree_checklist.md` in the package root) to implement a new type of tree. Charcoal aims to be the catch-all crate for all types of trees, so feel free to submit a direct PR to add your tree type instead of publishing your own Charcoal-based crate.

[`Error`]: https://doc.rust-lang.org/std/error/trait.Error.html " "
[`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html " "
[`VecDeque`]: https://doc.rust-lang.org/std/collections/struct.VecDeque.html " "
[`SmallVec`]: https://docs.rs/smallvec/*/smallvec/struct.SmallVec.html " "
[`ArrayVec`]: https://docs.rs/arrayvec/*/arrayvec/struct.ArrayVec.html " "
[`SlotMap`]: https://docs.rs/slotmap/*/slotmap/struct.SlotMap.html " "
[`HopSlotMap`]: https://docs.rs/slotmap/*/slotmap/hop/struct.HopSlotMap.html " "
[`DenseSlotMap`]: https://docs.rs/slotmap/*/slotmap/dense/struct.DenseSlotMap.html " "
[tracking issue for RFC 2514]: https://github.com/rust-lang/rust/issues/55149 " "
[arena tree blog post]: https://dev.to/deciduously/no-more-tears-no-more-knots-arena-allocated-trees-in-rust-44k6 " "
