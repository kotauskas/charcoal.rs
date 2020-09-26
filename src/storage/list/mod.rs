#[cfg(feature = "alloc")]
mod alloc_impl;
#[cfg(feature = "arrayvec")]
mod arrayvec_impl;
#[cfg(feature = "smallvec")]
mod smallvec_impl;

mod sparse;
pub use sparse::{
    SparseStorage,
    Slot as SparseStorageSlot,
    Vec as SparseVec,
    VecDeque as SparseVecDeque,
};

use core::num::{NonZeroUsize, NonZeroIsize};
use super::Storage;

const U_ONE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1) };

/// Trait for list-like containers which can be the backing storage for trees.
///
/// # Safety
/// There's a number of invariants which have to be followed by the container:
/// - The length of the storage cannot be modified in the container when it's borrowed immutably or not borrowed at all;
/// - `new` and `with_capacity` ***must*** return empty storages, i.e. those which have `len() == 0` and `is_empty() == true`;
/// - it should be impossible for the length of the storage to overflow `usize`;
/// - Calling [`get_unchecked`] or [`get_unchecked_mut`] with `self.len() > index` should *not* cause undefined behavior (otherwise, it may or may not — that is implementation specific);
/// - `insert_and_fix`/`remove_and_fix` call unsafe methods from [`MoveFix`], meaning that `insert` and `remove` must be implemented according to the contract of the methods of that trait;
/// - If an element is added at a position, it must be retrieveable in the exact same state as it was inserted until it is removed or modified using a method which explicitly does so.
///
/// Tree structures may rely on those invariants for safety.
///
/// [`get_unchecked`]: #method.get_unchecked " "
/// [`get_unchecked_mut`]: #method.get_unchecked_mut " "
/// [`MoveFix`]: trait.MoveFix.html " "
pub unsafe trait ListStorage: Sized {
    /// The type of values in the container.
    type Element;

    /// Creates an empty collection with the specified capacity.
    ///
    /// # Panics
    /// Collections with a fixed capacity should panic if the specified capacity does not match their actual one, and are recommended to override the `new` method to use the correct capacity.
    fn with_capacity(capacity: usize) -> Self;
    /// Inserts an element at position `index` within the collection, shifting all elements after it to the right.
    ///
    /// # Panics
    /// Required to panic if `index > len()`.
    fn insert(&mut self, index: usize, element: Self::Element);
    /// Removes and returns the element at position `index` within the vector, shifting all elements after it to the left.
    ///
    /// # Panics
    /// Required to panic if the specified index does not exist.
    fn remove(&mut self, index: usize) -> Self::Element;
    /// Returns the number of elements in the collection, also referred to as its 'length'.
    fn len(&self) -> usize;
    /// Returns a reference to the specified element in the collection, without doing bounds checking.
    ///
    /// # Safety
    /// If the specified index is out of bounds, a dangling reference will be created, causing *immediate undefined behavior*.
    unsafe fn get_unchecked(&self, index: usize) -> &Self::Element;
    /// Returns a *mutable* reference to the specified element in the collection, without doing bounds checking.
    ///
    /// # Safety
    /// If the specified index is out of bounds, a dangling reference will be created, causing *immediate undefined behavior*.
    unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Self::Element;

    /// Returns a reference to the specified element in the collection, or `None` if the index is out of bounds.
    #[inline]
    fn get(&self, index: usize) -> Option<&Self::Element> {
        if self.len() > index {
            Some(unsafe {
                // SAFETY: we just did a bounds check
                self.get_unchecked(index)
            })
        } else {
            None
        }
    }
    /// Returns a *mutable* reference to the specified element in the collection, or `None` if the index is out of bounds.
    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Element> {
        if self.len() > index {
            Some(unsafe {
                // SAFETY: we just did a bounds check
                self.get_unchecked_mut(index)
            })
        } else {
            None
        }
    }
    /// Creates a new empty collection. Dynamically-allocated collections created this way do not allocate memory.
    ///
    /// Collections with fixed capacity should override this method to use the correct capacity, as the default implementation calls `Self::with_capacity(0)`.
    #[inline(always)]
    fn new() -> Self {
        Self::with_capacity(0)
    }
    /// Returns `true` if the collection contains no elements, `false` otherwise.
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Appends an element to the back of the collection.
    #[inline(always)]
    fn push(&mut self, element: Self::Element) {
        self.insert(self.len(), element)
    }
    /// Removes the last element from the collection and returns it, or `None` if it is empty.
    #[inline]
    fn pop(&mut self) -> Option<Self::Element> {
        if !self.is_empty() {
            Some(self.remove(self.len() - 1))
        } else {
            None
        }
    }
    /// Returns the amount of elements the collection can hold without requiring a memory allocation.
    ///
    /// For collections which have a fixed capacity, this should be equal to the length; the default implementation uses exactly that.
    #[inline(always)]
    fn capacity(&self) -> usize {
        self.len()
    }
    /// Reserves capacity for at least additional more elements to be inserted in the given collection. The collection may reserve more space to avoid frequent reallocations. After calling `reserve`, `capacity` will be greater than or equal to `self.len()` + `additional`. Does nothing if capacity is already sufficient.
    ///
    /// For collections which have a fixed capacity, this should first check for the specified amount of elements to reserve for and if it's not zero, either reallocate the collection anew or, if that is not supported, panic. The default implementation does exactly that.
    #[inline(always)]
    fn reserve(&mut self, additional: usize) {
        if self.len() + additional > self.capacity() {
            unimplemented!("this storage type does not support reallocation")
        }
    }
    /// Shrinks the capacity of the collection as much as possible.
    ///
    /// It will drop down as close as possible to the current length, though dynamically allocated collections may not always reallocate exactly as much as it is needed to store all elements and none more.
    ///
    /// The default implementation does nothing.
    #[inline(always)]
    fn shrink_to_fit(&mut self) {}
    /// Shortens the collection, keeping the first `len` elements and dropping the rest.
    ///
    /// If `len` is greater than the collection's current length, this has no effect.
    ///
    /// Note that this method has no effect on the allocated capacity of the collection.
    fn truncate(&mut self, len: usize) {
        let current_length = self.len();
        if len > current_length || current_length == 0 {
            return;
        }
        for i in (current_length - 1)..=len {
            self.remove(i);
        }
    }
    /// Inserts an element at position `index` within the collection. The items after the inserted item should be notified using the [`MoveFix`] trait or not have their indices changed at all (index changes are not guaranteed and this behavior is implementation-dependent).
    ///
    /// # Panics
    /// Same as `insert`.
    ///
    /// [`MoveFix`]: trait.MoveFix.html " "
    #[inline]
    fn insert_and_shiftfix(&mut self, index: usize, element: Self::Element)
    where Self::Element: MoveFix,
    {
        self.insert(index, element);
        /*let len = self.len();
        let fix_start_index = index + 1; // Don't fix the new element
        if fix_start_index >= len {
            // We inserted at the end — don't need to fix anything
            return;
        }
        for i in fix_start_index..len {
            unsafe {
                Self::Element::fix_right_shift(self, i, index, 1);
            }
        }*/
        unsafe {
            // SAFETY: here we assume that removal actually did its job
            // (since the trait is unsafe, we can make this assumption)
            Self::Element::fix_right_shift(self, index, U_ONE);
        }
    }
    /// Removes and returns the element at position `index` within the collection. The items after the inserted item should be notified using the [`MoveFix`] trait or not change indicies at all (index changes are not guaranteed and this behavior is implementation-dependent).
    ///
    /// # Panics
    /// Same as `remove`.
    ///
    /// [`MoveFix`]: trait.MoveFix.html " "
    #[inline]
    fn remove_and_shiftfix(&mut self, index: usize) -> Self::Element
    where Self::Element: MoveFix,
    {
        let element = self.remove(index);
        /*for i in index..self.len() {
            unsafe {
                Self::Element::fix_left_shift(self, i, index, 1);
            }
        }*/
        unsafe {
            // SAFETY: as above
            Self::Element::fix_left_shift(self, index, U_ONE);
        }
        element
    }
    /// Adds an element to the collection at an arbitrary index, returning that index. Will never shift elements around. The default implementation will call `push` and return the index of the element pushed.
    ///
    /// This method is used instead of `push` by data structures. It is overriden by `SparseStorage` with the use of a free-list for placing new elements in place of old holes.
    #[inline(always)]
    fn add(&mut self, element: Self::Element) -> usize {
        self.push(element);
        self.len() - 1
    }
}
unsafe impl<T, E> Storage for T
where
    T: ListStorage<Element = E>,
    E: MoveFix,
{
    type Key = usize;
    type Element = E;

    #[inline(always)]
    fn add(&mut self, element: Self::Element) -> usize {
        <Self as ListStorage>::add(self, element)
    }
    #[inline(always)]
    fn remove(&mut self, index: &usize) -> Self::Element {
        <Self as ListStorage>::remove_and_shiftfix(self, *index)
    }
    #[inline(always)]
    fn len(&self) -> usize {
        <Self as ListStorage>::len(self)
    }
    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        <Self as ListStorage>::with_capacity(capacity)
    }
    #[inline(always)]
    unsafe fn get_unchecked(&self, index: &usize) -> &Self::Element {
        <Self as ListStorage>::get_unchecked(self, *index)
    }
    #[inline(always)]
    unsafe fn get_unchecked_mut(&mut self, index: &usize) -> &mut Self::Element {
        <Self as ListStorage>::get_unchecked_mut(self, *index)
    }
    #[inline(always)]
    fn contains_key(&self, index: &usize) -> bool {
        <Self as ListStorage>::len(self) > *index
    }
    #[inline(always)]
    fn get(&self, index: &usize) -> Option<&Self::Element> {
        <Self as ListStorage>::get(self, *index)
    }
    #[inline(always)]
    fn get_mut(&mut self, index: &usize) -> Option<&mut Self::Element> {
        <Self as ListStorage>::get_mut(self, *index)
    }
    #[inline(always)]
    fn new() -> Self {
        Self::with_capacity(0)
    }
    #[inline(always)]
    fn capacity(&self) -> usize {
        <Self as ListStorage>::capacity(self)
    }
    #[inline(always)]
    fn reserve(&mut self, additional: usize) {
        <Self as ListStorage>::reserve(self, additional)
    }
    #[inline(always)]
    fn shrink_to_fit(&mut self) {
        <Self as ListStorage>::shrink_to_fit(self)
    }
}

/// Trait for tree node types to be able to correct their parent/child node indices when elements are moved around in the collection.
///
/// See the documentation on the individual methods for more details on the semantics of those hooks.
pub trait MoveFix: Sized {
    /// The hook to be called when the items in the collection get shifted due to an insertion or removal. `shifted_from` specifies the index from which the shift starts (first affected element), i.e. the index at which a new item was inserted or from which an item was removed. Positive values for `shifted_by` indicate a shift to the right, negative are to the left.
    ///
    /// This method is *never* called directly by storages. `fix_left_shift` and `fix_right_shift` are called instead. See those for more on how and when this method gets called.
    ///
    /// # Safety
    /// This method can ***only*** be called by `fix_left_shift` and `fix_right_shift`. All safety implications of those methods apply.
    unsafe fn fix_shift<S>(storage: &mut S, shifted_from: usize, shifted_by: NonZeroIsize)
    where S: ListStorage<Element = Self>;
    /// The hook to be called when an element in a collection gets moved from one location to another. `previous_index` represents its previous index before moving and is *not* guaranteed to be a valid index, `current_index` is its new index and is guaranteed to point towards a valid element.
    ///
    /// # Safety
    /// The implementor of this method may cause undefined behavior if the method was called erroneously and elements were not actually swapped.
    ///
    /// [`SparseStorage`]: struct.SparseStorage.html " "
    unsafe fn fix_move<S>(storage: &mut S, previous_index: usize, current_index: usize)
    where S: ListStorage<Element = Self>;
    /// The hook to be called when the items in the collection get shifted to the *left* due to a *removal*. `shifted_from` specifies the index from which the shift starts (first affected element), i.e. the index from which an item was removed.
    ///
    /// **The method gets called on the `shifted_from` element and all elements after it, in order; it's not called on elements before that point.** For tree elements, this means that if this method got called on them, they not only need to fix their child nodes' indicies, they also need to fix the indicies of their parents that point towards themselves.
    ///
    /// # Safety
    /// The implementor of this method may cause undefined behavior if the method was called erroneously and elements were not actually shifted.
    #[inline(always)]
    unsafe fn fix_left_shift<S>(storage: &mut S, shifted_from: usize, shifted_by: NonZeroUsize)
    where S: ListStorage<Element = Self>,
    {
        Self::fix_shift(
            storage,
            shifted_from,
            NonZeroIsize::new((shifted_by.get() as isize).wrapping_neg())
                .expect("unexpected integer overflow"),
        );
    }
    /// The hook to be called when the items in the collection get shifted to the *right* due to an *insertion*. `shifted_from` specifies the index from which the shift starts (first affected element), i.e. the index at which a new item was inserted.
    ///
    /// **The method gets called on all elements *after* the `shifted_from` one, in order; it's not called on elements before that point.** For tree elements, this means that if this method got called on them, they not only need to fix their child nodes' indicies, they also need to fix the indicies of their parents that point towards themselves.
    ///
    /// # Safety
    /// The implementor of this method may cause undefined behavior if the method was called erroneously and elements were not actually shifted.
    #[inline(always)]
    unsafe fn fix_right_shift<S>(storage: &mut S, shifted_from: usize, shifted_by: NonZeroUsize)
    where S: ListStorage<Element = Self>,
    {
        Self::fix_shift(
            storage,
            shifted_from,
            NonZeroIsize::new(shifted_by.get() as isize).expect("unexpected integer overflow"),
        );
    }
}
