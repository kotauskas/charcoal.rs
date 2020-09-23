use core::{
    ptr,
    mem,
    num::NonZeroUsize,
    hint,
};
use super::{ListStorage, MoveFix};

/// A `Vec` wrapped in [`SparseStorage`].
///
/// [`SparseStorage`]: struct.SparseStorage.html " "
#[cfg(feature = "alloc")]
pub type Vec<T> = SparseStorage<T, alloc::vec::Vec<Slot<T>>>;
/// A `VecDeque` wrapped in [`SparseStorage`].
///
/// [`SparseStorage`]: struct.SparseStorage.html " "
#[cfg(feature = "alloc")]
pub type VecDeque<T> = SparseStorage<T, alloc::collections::VecDeque<Slot<T>>>;

/// A wrapper around a list-like storage type which considerably improves performance when removing elements.
///
/// Sparse storage with element type `E` wraps a normal storage which stores `Slot<E>`, which is a tagged union storing either an element or a "hole". Those holes count as regular elements, but trying to get their value produces a panic, since the storage provides `E` as its element type, rather than `Slot<E>`. This behavior does not depend on whether checked or unchecked `get`/`get_mut` methods are used - all of those are guaranteed to panic upon fetching a hole.
///
/// When `remove_and_shiftfix` is called, elements are not actually shifted, but the element is replaced with a hole. If the elements of the storage store indicies towards other elements of the storage, they don't get invalidated.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SparseStorage<E, S>
where S: ListStorage<Element = Slot<E>> {
    storage: S,
    /// Length, first element, last element
    hole_list: Option<(NonZeroUsize, usize, usize)>,
}
impl<E, S> SparseStorage<E, S>
where S: ListStorage<Element = Slot<E>> {
    /// Removes all holes from the sparse storage, *without fixing elements' indicies*. **This is an expensive operation and should only be called if `is_dense` is `false` to avoid needless overhead.**
    #[inline(always)]
    pub fn defragment(&mut self) {
        self.defragment_impl(|_, _, _| {});
    }
    /// Removes all holes from the sparse storage, fixing elements' indicies. **This is an expensive operation and should only be called if `is_dense` is `false` to avoid needless overhead.**
    #[inline(always)]
    pub fn defragment_and_fix(&mut self)
    where E: MoveFix {
        self.defragment_impl(|s, i, j| {
            unsafe {
                // SAFETY: we just swapped those elements
                E::fix_move(s, i, j);
            }
        });
    }
    fn defragment_impl<F>(&mut self, mut f: F)
    where F: FnMut(&mut Self, usize, usize) {
        let hole_info = if let Some(val) = self.hole_list {
            val
        } else {
            // No holes == nothing to defragment
            return;
        };
        for i in 0..self.len() {
            let element = unsafe {
                // SAFETY: get_unchecked_mut with index < len is always safe
                self.storage.get_unchecked_mut(i)
            };
            let element_is_hole = element.is_hole();
            let element = element as *mut _;
            if element_is_hole {
                'j: for j in (0..self.len()).rev() {
                    if i == j {
                        // Don't move holes back to the beginning
                        break 'j;
                    }
                    let other_element = unsafe {
                        // SAFETY: as above
                        self.storage.get_unchecked_mut(j)
                    };
                    if other_element.is_element() {
                        unsafe {
                            // SAFETY: both pointers were created from references, meaning that
                            // they can't overlap or be invalid
                            ptr::swap_nonoverlapping(element, other_element as *mut _, 1);
                        }
                        f(self, i, j);
                    }
                }
            }
        }
        for (_, _) in (0..self.len()).rev().zip(0..hole_info.0.get()) {
            // We don't need to check for holes at this point, since we're already checking by
            // the number of them
            self.storage.pop();
        }
        // We popped off all holes, thus nothing to point at
        self.hole_list = None;
    }
    /// Consumes the sparse storage and returns its inner storage.
    #[inline(always)]
    pub fn into_inner(self) -> S {
        self.storage
    }
    /// Returns the number of holes in the storage. This operation returns immediately instead of looping through the entire storage, since the sparse storage automatically tracks the number of holes it creates and destroys.
    #[inline(always)]
    pub fn num_holes(&self) -> usize {
        self.hole_list.map_or(0, |x| x.0.get())
    }
    /// Returns `true` if there are no holes in the storage, `false` otherwise. This operation returns immediately instead of looping through the entire storage, since the sparse storage automatically tracks the number of holes it creates and destroys.
    #[inline(always)]
    pub fn is_dense(&self) -> bool {
        self.num_holes() == 0
    }

    /// Sets the specified element to a hole, returning the value or `None` if it was already a hole.
    ///
    /// # Safety
    /// The specified index must be within range. Hole info must not point to non-holes.
    unsafe fn punch_hole(&mut self, index: usize) -> Option<E> {
        let element = /*unsafe*/ {
            // SAFETY: see safety contract
            self.storage.get_unchecked_mut(index)
        };
        element.punch_hole(None).map(move |val| {
            if let Some(mut hole_info) = self.hole_list {
                hole_info.0 = /*unsafe*/ {
                    // SAFETY: it's impossible to have more than usize::MAX elements in a Storage
                    let mut raw = hole_info.0.get();
                    raw += 1;
                    NonZeroUsize::new_unchecked(raw)
                };
                let old_end = /*unsafe*/ {
                    // SAFETY: as above
                    self.storage.get_unchecked_mut(hole_info.2)
                };
                /*unsafe*/ {
                    // SAFETY: hole info cannot point to non-holes
                    // Make the previous end point to the hole we just punched
                    old_end.set_hole_link(Some(index));
                }
                // Set end to the hole punched
                hole_info.2 = index;
            } else {
                self.hole_list = Some((
                    /*unsafe*/ {
                        // SAFETY: self explanatory
                        NonZeroUsize::new_unchecked(1)
                    }, // Only one hole
                    index, // List starts from the hole we just punched...
                    index, // ...and ends with it
                ));
            }
            val
        })
    }
}
static HOLE_PANIC_MSG: &str = " \
the element at the specified index was a hole in the sparse storage";
unsafe impl<E, S> ListStorage for SparseStorage<E, S>
where S: ListStorage<Element = Slot<E>> {
    type Element = E;

    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        Self {storage: S::with_capacity(capacity), hole_list: None}
    }
    #[inline(always)]
    fn insert(&mut self, index: usize, element: Self::Element) {
        // Normal inserts ignore holes
        self.storage.insert(index, Slot::new_element(element))
    }
    #[inline]
    fn remove(&mut self, index: usize) -> Self::Element {
        if self.is_dense() {
            self.storage.remove(index).unwrap()
        } else {
            unimplemented!("\
cannot perform raw removal from sparse storage without defragmenting, use remove_and_shiftfix or \
defragment before doing this")
        }
    }
    #[inline(always)]
    fn len(&self) -> usize {
        self.storage.len()
    }
    #[inline]
    unsafe fn get_unchecked(&self, index: usize) -> &Self::Element {
        self.storage.get_unchecked(index).element_checked().expect(HOLE_PANIC_MSG)
    }
    #[inline]
    unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Self::Element {
        self.storage.get_unchecked_mut(index).element_checked_mut().expect(HOLE_PANIC_MSG)
    }

    #[inline]
    #[track_caller]
    fn get(&self, index: usize) -> Option<&Self::Element> {
        match self.storage.get(index) {
            Some(x) if x.is_element() => unsafe {
                // SAFETY: guarded match arm checks for holes
                Some(x.element())
            },
            Some(..) => panic!(HOLE_PANIC_MSG),
            None => None,
        }
    }
    #[inline]
    #[track_caller]
    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Element> {
        match self.storage.get_mut(index) {
            Some(x) if x.is_element() => unsafe {
                // SAFETY: as above
                Some(x.element_mut())
            },
            Some(..) => panic!(HOLE_PANIC_MSG),
            None => None,
        }
    }
    #[inline(always)]
    fn new() -> Self {
        Self {storage: S::new(), hole_list: None}
    }
    #[inline(always)]
    fn push(&mut self, element: Self::Element) {
        self.storage.push(Slot::new_element(element))
    }
    #[inline]
    fn pop(&mut self) -> Option<Self::Element> {
        if self.is_dense() {
            self.storage.pop().map(Slot::unwrap)
        } else {
            unimplemented!("\
cannot perform raw removal from sparse storage without defragmenting, use remove_and_shiftfix or \
defragment before doing this")
        }
    }
    #[inline(always)]
    fn capacity(&self) -> usize {
        self.storage.capacity()
    }
    #[inline(always)]
    fn reserve(&mut self, additional: usize) {
        self.storage.reserve(additional)
    }
    #[inline(always)]
    fn shrink_to_fit(&mut self) {
        self.storage.shrink_to_fit()
    }
    #[inline(always)]
    fn truncate(&mut self, len: usize) {
        self.storage.truncate(len)
    }
    fn insert_and_shiftfix(&mut self, index: usize, element: Self::Element)
    where Self::Element: MoveFix {
        // We are not using holes here since the hole list isn't doubly linked and we might end up
        // pointing to an element
        self.insert(index, element);
        unsafe {
            // SAFETY: we are indeed shifting
            Self::Element::fix_right_shift(self, index, super::U_ONE);
        }
    }
    #[inline]
    #[track_caller]
    fn remove_and_shiftfix(&mut self, index: usize) -> Self::Element
    where Self::Element: MoveFix {
        assert!(self.len() > index, "index out of bounds");
        unsafe {
            // SAFETY: we just did bounds checking
            self.punch_hole(index)
        }.expect(HOLE_PANIC_MSG)
    }
    #[inline]
    #[allow(clippy::option_if_let_else)] // I hate map_or_else
    fn add(&mut self, element: Self::Element) -> usize {
        if let Some(hole_info) = &mut self.hole_list {
            let new_hole_count = NonZeroUsize::new(hole_info.0.get() - 1);
            let used_hole_index = hole_info.1;
            let hole = unsafe {
                // SAFETY: hole info always points within bounds
                self.storage.get_unchecked_mut(used_hole_index)
            };
            let next_hole = unsafe {
                // SAFETY: hole info always points to holes
                hole.hole_link()
            };
            *hole = Slot::new_element(element);
            if let Some(new_hole_count) = new_hole_count {
                hole_info.0 = new_hole_count;
                hole_info.1 = next_hole.unwrap_or_else(|| unsafe {
                    // SAFETY: according to hole count, the hole list cannot end here
                    hint::unreachable_unchecked()
                });
            } else {
                self.hole_list = None;
            }
            used_hole_index
        } else {
            self.push(element);
            self.len() - 1
        }
    }
}

/// A slot inside a sparse storage.
///
/// This is an opaque structure, only used for the purpose of a `SparseStorage` being validly declarable, because leaking private types through generic argument defaults is impossible, and it'd be impossible to declare the type of the backing storage if it was explicitly different.
///
/// # Size and representation
/// *The contents of this section are an implementation detail. Unless stated otherwise, relying on those for memory safety may cause undefined behavior.*
///
/// The structure is actually a newtype wrapper around an concrete implementation for storing the value.
///
/// ## Union version
/// If the `union_optimizations` feature flag is enabled, the layout looks like this:
/// ```no_run
/// # /*
/// struct SlotUnionBased<T> {
///     discrim: u8,
///     data: SlotUnion<T>,
/// }
/// union SlotUnion<T> {
///     hole_link: usize,
///     element: T,
/// }
/// # */
/// ```
/// The `hole_link` member of the union is actually `Option<usize>` under the hood. If `None`, the `0b0000_0010` bit in `discrim` is set to zero; otherwise, it is set to 1. For soundness purposes, the value of `hole_link` is never uninitialized and is instead set to an arbitrary value when it's supposed to be `None`.
///
/// ### Exact size and alignment
/// The following members contribute to size:
/// - either `usize` (1 pointer) or `T` (arbitrary size)
/// - discriminant: one `u8` (1 byte)
/// - padding: 1 pointer minus 1 byte for the discriminant to fit in, can be more due to the alignment of `T`
///
/// **Total size:** *2 pointers* (*16 bytes* on 64-bit systems, *8 bytes* on 32-bit systems) or more depending on the size of `T` *if it's over the size of* ***1 pointer***
/// **Total alignment:** the same as a *pointer* (largest primitive alignment), but may be more if `T` specifies a bigger exotic alignment explicitly
///
/// ## Enum version
/// If the `union_optimizations` feature flag is disabled (always the case on the current stable compiler *as of Rust 1.46*), the following enum-based representation is used instead:
/// ```no_run
/// # /*
/// enum SlotEnumBased<T> {
///    Element(T),
///    Hole(Option<usize>),
///}
/// # */
/// ```
///
/// ### Exact size and alignment
/// The following members contribute to size:
/// - either `Option<usize>` (2 pointers: one for the value, another one for discriminant and alignment) or `T` (arbitrary size)
/// - discriminant and padding: at least 1 pointer wide, can be more due to the alignment of `T`
///
/// **Total size:** *3 pointers* (*24 bytes* on 64-bit systems, *12 bytes* on 32-bit systems) or more depending on the size of `T` *if it's over the size of* ***2 pointers***
/// **Total alignment:** the same as a *pointer* (largest primitive alignment), but may be more if `T` specifies a bigger exotic alignment explicitly
#[repr(transparent)]
#[derive(Debug, Hash)]
pub struct Slot<T> (SlotInner<T>);
impl<T> Slot<T> {
    #[inline(always)]
    const fn new_element(val: T) -> Self {
        Self (SlotInner::new_element(val))
    }
    // Uncomment if ever needed
    /*#[inline(always)]
    const fn new_hole(val: Option<usize>) -> Self {
        Self (SlotInner::new_hole(val))
    }*/
    #[inline(always)]
    const fn is_element(&self) -> bool {
        self.0.is_element()
    }
    #[inline(always)]
    const fn is_hole(&self) -> bool {
        self.0.is_hole()
    }
    #[inline(always)]
    unsafe fn element(&self) -> &T {
        self.0.element()
    }
    fn element_checked(&self) -> Option<&T> {
        if self.is_element() {
            unsafe {
                // SAFETY: we just checked for that
                Some(self.element())
            }
        } else {None}
    }
    #[inline(always)]
    unsafe fn element_mut(&mut self) -> &mut T {
        self.0.element_mut()
    }
    fn element_checked_mut(&mut self) -> Option<&mut T> {
        if self.is_element() {
            unsafe {
                // SAFETY: we just checked for that
                Some(self.element_mut())
            }
        } else {None}
    }
    #[inline(always)]
    unsafe fn hole_link(&self) -> Option<usize> {
        self.0.hole_link()
    }
    #[inline(always)]
    unsafe fn set_hole_link(&mut self, val: Option<usize>) {
        self.0.set_hole_link(val)
    }
    #[inline]
    #[track_caller]
    fn unwrap(self) -> T {
        if self.is_element() {
            let element_owned = unsafe {
                // SAFETY: self is a valid reference and we're doing mem::forget() on self
                ptr::read(self.element())
            };
            mem::forget(self);
            element_owned
        } else {
            panic!(HOLE_PANIC_MSG)
        }
    }
    #[inline(always)]
    fn punch_hole(&mut self, next: Option<usize>) -> Option<T> {
        self.0.punch_hole(next)
    }
}

#[cfg(feature = "union_optimizations")]
type SlotInner<T> = SlotUnionBased<T>;
#[cfg(not(feature = "union_optimizations"))]
type SlotInner<T> = SlotEnumBased<T>;

#[cfg(feature = "union_optimizations")]
struct SlotUnionBased<T> {
    // Bit 0 is union discriminant (0 is hole, 1 is element), bit 1 is hole link discriminant
    discrim: u8,
    data: SlotUnion<T>,
}
#[cfg(feature = "union_optimizations")]
impl<T> SlotUnionBased<T> {
    const HOLE_DISCRIM_BIT: u8 = 0b0000_0000;
    const ELEMENT_DISCRIM_BIT: u8 = 0b0000_0001;

    const UNION_DISCRIM_MASK: u8 = 0b0000_0001;
    const LINK_DISCRIM_MASK: u8 = 0b0000_0010;

    #[inline(always)]
    const fn new_element(val: T) -> Self {
        Self {
            discrim: Self::ELEMENT_DISCRIM_BIT,
            data: SlotInner {
                element: val,
            },
        }
    }
    // Uncomment if ever needed
    #[inline(always)]
    const fn new_hole(val: Option<usize>) -> Self {
        Self {
            discrim: Self::HOLE_DISCRIM_BIT | ((val.is_some() as u8) << 1),
            data: SlotInner {
                hole_link: val.unwrap_or_default(), // Uninit integers are unsound
            }
        }
    }
    #[inline(always)]
    const fn is_element(&self) -> bool {
        self.discrim & Self::UNION_DISCRIM_MASK == Self::ELEMENT_DISCRIM_BIT
    }
    #[inline(always)]
    const fn is_hole(&self) -> bool {
        self.discrim & Self::UNION_DISCRIM_MASK == Self::HOLE_DISCRIM_BIT
    }
    #[inline(always)]
    const unsafe fn element(&self) -> &T {
        &self.data.element
    }
    #[inline(always)]
    unsafe fn element_mut(&mut self) -> &mut T {
        &mut self.data.element
    }
    #[inline]
    const unsafe fn hole_link(&self) -> Option<usize> {
        if self.discrim & Self::LINK_DISCRIM_MASK != 0 {
            Some(self.data.hole_link)
        } else {
            None
        }
    }
    #[inline(always)]
    unsafe fn set_hole_link(&mut self, val: Option<usize>) {
        let link_bit = (val.is_some() as u8) << 1;
        self.discrim = (self.discrim & Self::UNION_DISCRIM_MASK) | link_bit;
        self.data.hole_link = val.unwrap_or_default(); // Uninit integers are unsound
    }
    #[inline]
    fn punch_hole(&mut self, next: Option<usize>) -> Option<T> {
        match self.discrim & Self::UNION_DISCRIM_MASK {
            Self::ELEMENT_DISCRIM_BIT => {
                let val_owned = unsafe {
                    // SAFETY: the pointer is obtained from a reference and therefore is
                    // guranteed to be valid; the value will not be duplicated because
                    // we're overwriting it right after this operation
                    ptr::read(&mut self.data.element)
                };
                unsafe {
                    // SAFETY: as above
                    ptr::write(self, Self::new_hole(next));
                }
                Some(val_owned)
            },
            Self::HOLE_DISCRIM_BIT => None,
            _ => unsafe {
                // SAFETY: we're masking out one bit and matching it, other values
                // can't possibly appear
                hint::unreachable_unchecked()
            },
        }
    }
}
#[cfg(feature = "union_optimizations")]
union SlotUnion<T> {
    hole_link: usize,
    element: T,
}

#[cfg(not(feature = "union_optimizations"))]
#[derive(Debug, Hash)]
enum SlotEnumBased<T> {
    /// A value in the slot.
    Element(T),
    /// A hole, with an index to the next one.
    Hole(Option<usize>),
}
#[cfg(not(feature = "union_optimizations"))]
impl<T> SlotEnumBased<T> {
    #[inline(always)]
    const fn new_element(val: T) -> Self {
        Self::Element(val)
    }
    // Uncomment if ever needed
    /*#[inline(always)]
    const fn new_hole(val: Option<usize>) -> Self {
        Self::Hole(val)
    }*/
    #[inline(always)]
    const fn is_element(&self) -> bool {
        matches!(self, Self::Element(..))
    }
    #[inline(always)]
    const fn is_hole(&self) -> bool {
        matches!(self, Self::Hole(..))
    }
    #[inline(always)]
    #[allow(clippy::missing_const_for_fn)] // unreachable_unchecked isn't stable as const fn
    unsafe fn element(&self) -> &T {
        match self {
            Self::Element(x) => x,
            Self::Hole(..) => hint::unreachable_unchecked(),
        }
    }
    #[inline(always)]
    unsafe fn element_mut(&mut self) -> &mut T {
        match self {
            Self::Element(x) => x,
            Self::Hole(..) => hint::unreachable_unchecked(),
        }
    }
    #[inline]
    #[allow(clippy::missing_const_for_fn)] // unreachable_unchecked isn't stable as const fn
    unsafe fn hole_link(&self) -> Option<usize> {
        match self {
            Self::Hole(x) => *x,
            Self::Element(..) => hint::unreachable_unchecked(),
        }
    }
    #[inline(always)]
    unsafe fn set_hole_link(&mut self, val: Option<usize>) {
        match self {
            Self::Hole(x) => {
                *x = val;
            },
            Self::Element(..) => hint::unreachable_unchecked(),
        }
    }
    fn punch_hole(&mut self, next: Option<usize>) -> Option<T> {
        match self {
            Self::Element(val) => {
                let val_owned = unsafe {
                    // SAFETY: the pointer is obtained from a reference and therefore is
                    // guranteed to be valid; the value will not be duplicated because
                    // we're overwriting it right after this operation
                    ptr::read(val)
                };
                unsafe {
                    // SAFETY: as above
                    ptr::write(self, Self::Hole(next));
                }
                Some(val_owned)
            },
            Self::Hole(..) => None,
        }
    }
}