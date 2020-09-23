//! Utilities for treating the backing storage for trees generically.
//!
//! This module is home for the following items:
//! - [`Storage`], the main trait for the backing storages for trees
//! - [`ListStorage`], a trait used for implementing `Storage` for list-like collections
//! - [`SparseStorage`], a wrapper around `ListStorage`s which greatly improves element removal performance
//! - [`DefaultStorage`], a type definition for the default backing storage used by trees unless a different one is specified; takes different values depending on feature flags
//!
//! [`Storage`]: trait.Storage.html " "
//! [`ListStorage`]: trait.ListStorage.html " "
//! [`SparseStorage`]: struct.SparseStorage.html " "
//! [`DefaultStorage`]: type.DefaultStorage.html " "

mod list;
pub use list::*;

#[cfg(feature = "slotmap_storage")]
mod slotmap_impl;

use core::fmt::Debug;

/// Trait for various kinds of containers which can be the backing storage for trees.
///
/// # Safety
/// There's a number of invariants which have to be followed by the container:
/// - The length of the storage cannot be modified in the container when it's borrowed immutably or not borrowed at all;
/// - `new` and `with_capacity` ***must*** return empty storages, i.e. those which have `len() == 0` and `is_empty() == true`;
/// - it should be impossible for the length of the storage to overflow `usize`;
/// - Calling [`get_unchecked`] or [`get_unchecked_mut`] if `contains_key` on the same key returns `true` should *not* cause undefined behavior (otherwise, it may or may not — that is implementation specific);
/// - Calling `remove` if `contains_key` on the same key should *never* panic, as that might leave the tree in an invalid state during some operations;
/// - If an element is added at a key, it must be retrieveable in the exact same state as it was inserted until it is removed or modified using a method which explicitly does so.
///
/// Tree structures may rely on those invariants for safety.
pub unsafe trait Storage: Sized {
    /// The type used for element naming.
    type Key: Clone + Debug + Eq;
    /// The type of the elements stored.
    type Element;

    /// Adds an element to the collection with an unspecified key, returning that key.
    fn add(&mut self, element: Self::Element) -> Self::Key;
    /// Removes and returns the element identified by `key` within the storage.
    ///
    /// # Panics
    /// Required to panic if the specified key does not exist.
    fn remove(&mut self, key: &Self::Key) -> Self::Element;
    /// Returns the number of elements in the storage, also referred to as its 'length'.
    fn len(&self) -> usize;
    /// Creates an empty storage with the specified capacity.
    ///
    /// # Panics
    /// Storages with a fixed capacity should panic if the specified capacity does not match their actual one, and are recommended to override the `new` method to use the correct capacity.
    fn with_capacity(capacity: usize) -> Self;
    /// Returns a reference to the specified element in the storage, without checking for presence of the key inside the collection.
    ///
    /// # Safety
    /// If the element at the specified key is not present in the storage, a dangling reference will be created, causing *immediate undefined behavior*.
    unsafe fn get_unchecked(&self, key: &Self::Key) -> &Self::Element;
    /// Returns a *mutable* reference to the specified element in the storage, without checking for presence of the key inside the collection.
    ///
    /// # Safety
    /// If the element at the specified key is not present in the storage, a dangling reference will be created, causing *immediate undefined behavior*.
    unsafe fn get_unchecked_mut(&mut self, key: &Self::Key) -> &mut Self::Element;
    /// Returns `true` if the specified key is present in the storage, `false` otherwise.
    ///
    /// If this method returned `true`, calling `get_unchecked`/`get_unchecked_mut` on the same key is guaranteed to be safe.
    fn contains_key(&self, key: &Self::Key) -> bool;

    /// Returns a reference to the specified element in the collection, or `None` if the key is not present in the storage.
    #[inline]
    fn get(&self, key: &Self::Key) -> Option<&Self::Element> {
        if self.contains_key(key) {
            Some(unsafe {
                // SAFETY: we just checked for key presence
                self.get_unchecked(key)
            })
        } else {
            None
        }
    }
    /// Returns a *mutable* reference to the specified element in the collection, or `None` if the key is not present in the storage.
    #[inline]
    fn get_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Element> {
        if self.contains_key(key) {
            Some(unsafe {
                // SAFETY: we just checked for key presence
                self.get_unchecked_mut(key)
            })
        } else {
            None
        }
    }
    /// Creates a new empty storage. Dynamically-allocated storages created this way do not allocate memory.
    ///
    /// Storages with fixed capacity should override this method to use the correct capacity, as the default implementation calls `Self::with_capacity(0)`.
    #[inline(always)]
    fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Returns `true` if the storage contains no elements, `false` otherwise.
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns the amount of elements the storage can hold without requiring a memory allocation.
    ///
    /// For storages which have a fixed capacity, this should be equal to the length; the default implementation uses exactly that.
    #[inline(always)]
    fn capacity(&self) -> usize {
        self.len()
    }
    /// Reserves capacity for at least additional more elements to be inserted in the given storage. The storage may reserve more space to avoid frequent reallocations. After calling `reserve`, `capacity` will be greater than or equal to `self.len()` + `additional`. Does nothing if capacity is already sufficient.
    ///
    /// For storages which have a fixed capacity, this should first check for the specified amount of elements to reserve for and if it's not zero, either reallocate the collection anew or, if that is not supported, panic. The default implementation does exactly that.
    #[inline(always)]
    fn reserve(&mut self, additional: usize) {
        if self.len() + additional > self.capacity() {
            unimplemented!("this storage type does not support reallocation")
        }
    }
    /// Shrinks the capacity of the storage as much as possible.
    ///
    /// It will drop down as close as possible to the current length, though dynamically allocated storages may not always reallocate exactly as much as it is needed to store all elements and none more.
    ///
    /// The default implementation does nothing.
    #[inline(always)]
    fn shrink_to_fit(&mut self) {}
}

/// The default storage type used by the tree types when a storage type is not provided.
///
/// This is chosen according to the following strategy:
/// - If the `alloc` feature flag is enabled, [`SparseVec`] is used
/// - If `alloc` is disabled but `smallvec_storage` is enabled, a [*sparse*][`SparseStorage`] [`SmallVec`] *with zero-sized backing storage* is used
/// - If both `smallvec_storage` and `alloc` are disabled, an [`ArrayVec`] *with zero-sized backing storage* is used
/// No other storage types are ever used as defaults.
///
/// [`SparseVec`]: type.SparseVec.html " "
/// [`SmallVec`]: https://docs.rs/smallvec/*/smallvec/struct.SmallVec.html " "
/// [`ArrayVec`]: https://docs.rs/arrayvec/*/arrayvec/struct.ArrayVec.html " "
/// [`SparseStorage`]: struct.SparseStorage.html " "
pub type DefaultStorage<T> = _DefaultStorage<T>;

#[cfg(feature = "alloc")]
type _DefaultStorage<T> = SparseVec<T>;

#[cfg(all(
    feature = "smallvec_storage",
    not(feature = "alloc"),
))]
type _DefaultStorage<T> = SparseStorage<smallvec::SmallVec<[T; 0]>, T>;

#[cfg(all(
    feature = "arrayvec_storage",
    not(feature = "alloc"),
    not(feature = "smallvec_storage"),
))]
type _DefaultStorage<T> = arrayvec::ArrayVec<[T; 0]>;

#[cfg(all(
    not(feature = "alloc"),
    not(feature = "smallvec_storage"),
    not(feature = "arrayvec_storage"),
))]
compile_error!("no default storage available, please enable one or more of the alloc, \
smallvec_storage or arrayvec_storage feature flags");