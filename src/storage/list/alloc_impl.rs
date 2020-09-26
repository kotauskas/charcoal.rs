use core::hint;
use alloc::{vec::Vec, collections::VecDeque};
use super::ListStorage;

unsafe impl<T> ListStorage for Vec<T> {
    type Element = T;

    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }
    #[inline(always)]
    fn insert(&mut self, index: usize, element: Self::Element) {
        self.insert(index, element)
    }
    #[inline(always)]
    fn remove(&mut self, index: usize) -> Self::Element {
        self.remove(index)
    }
    #[inline(always)]
    fn len(&self) -> usize {
        self.len()
    }
    #[inline(always)]
    unsafe fn get_unchecked(&self, index: usize) -> &Self::Element {
        (**self).get_unchecked(index)
    }
    #[inline(always)]
    unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Self::Element {
        (**self).get_unchecked_mut(index)
    }

    #[inline(always)]
    fn get(&self, index: usize) -> Option<&Self::Element> {
        (**self).get(index)
    }
    #[inline(always)]
    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Element> {
        (**self).get_mut(index)
    }
    #[inline(always)]
    fn new() -> Self {
        Self::new()
    }
    #[inline(always)]
    fn push(&mut self, element: Self::Element) {
        self.push(element)
    }
    #[inline(always)]
    fn pop(&mut self) -> Option<Self::Element> {
        self.pop()
    }
    #[inline(always)]
    fn capacity(&self) -> usize {
        self.capacity()
    }
    #[inline(always)]
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional)
    }
    #[inline(always)]
    fn shrink_to_fit(&mut self) {
        self.shrink_to_fit()
    }
    #[inline(always)]
    fn truncate(&mut self, len: usize) {
        self.truncate(len)
    }
}

unsafe impl<T> ListStorage for VecDeque<T> {
    type Element = T;

    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }
    #[inline(always)]
    fn insert(&mut self, index: usize, element: Self::Element) {
        self.insert(index, element)
    }
    #[inline(always)]
    fn remove(&mut self, index: usize) -> Self::Element {
        self.remove(index).expect("index out of bounds")
    }
    #[inline(always)]
    fn len(&self) -> usize {
        self.len()
    }
    #[inline(always)]
    unsafe fn get_unchecked(&self, index: usize) -> &Self::Element {
        // FIXME this relies on LLVM being smart enough to optimize out the bounds check
        self.get(index)
            .unwrap_or_else(|| hint::unreachable_unchecked())
    }
    #[inline(always)]
    unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Self::Element {
        // FIXME see above
        self.get_mut(index)
            .unwrap_or_else(|| hint::unreachable_unchecked())
    }

    #[inline(always)]
    fn get(&self, index: usize) -> Option<&Self::Element> {
        self.get(index)
    }
    #[inline(always)]
    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Element> {
        self.get_mut(index)
    }
    #[inline(always)]
    fn push(&mut self, element: Self::Element) {
        self.push_back(element)
    }
    #[inline(always)]
    fn pop(&mut self) -> Option<Self::Element> {
        self.pop_back()
    }
    #[inline(always)]
    fn capacity(&self) -> usize {
        self.capacity()
    }
    #[inline(always)]
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional)
    }
    #[inline(always)]
    fn shrink_to_fit(&mut self) {
        self.shrink_to_fit()
    }
    #[inline(always)]
    fn truncate(&mut self, len: usize) {
        self.truncate(len)
    }
}

/*
// TODO reimplement LinkedList with a custom SmartLinkedList
#[cfg(feature = "linked_list_storage")]
use alloc::collections::LinkedList;

#[cfg(feature = "linked_list_storage")]
unsafe impl<T> ListStorage for LinkedList<T> {
    type Element = T;

    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        assert_eq!(capacity, 0, "cannot create a linked list with nonzero preallocated capacity");
        Self::new()
    }
    #[inline(always)]
    fn insert(&mut self, index: usize, element: Self::Element) {
        assert!(index <= self.len(), "incorrect index");
        let mut cursor = {
            let mut cursor = self.cursor_front_mut();
            for _ in 0..index {
                cursor.move_next();
            }
            cursor
        };
        cursor.insert_after(element)
    }
    #[inline(always)]
    fn remove(&mut self, index: usize) -> Self::Element {
        let mut cursor = {
            let mut cursor = self.cursor_front_mut();
            for _ in 0..index {
                cursor.move_next();
            }
            cursor
        };
        cursor.remove_current().expect("index out of bounds")
    }
    #[inline(always)]
    fn len(&self) -> usize {
        self.len()
    }
    #[inline(always)]
    unsafe fn get_unchecked(&self, index: usize) -> &Self::Element {
        self.get(index).unwrap_or_else(|| unreachable_unchecked())
    }
    #[inline(always)]
    unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Self::Element {
        self.get_mut(index).unwrap_or_else(|| unreachable_unchecked())
    }
    #[inline(always)]
    fn get(&self, index: usize) -> Option<&Self::Element> {
        self.iter().nth(index)
    }
    #[inline(always)]
    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Element> {
        self.iter_mut().nth(index)
    }

    #[inline(always)]
    fn new() -> Self {
        Self::new()
    }
    #[inline(always)]
    fn push(&mut self, element: Self::Element) {
        self.push_back(element)
    }
    #[inline(always)]
    fn pop(&mut self) -> Option<Self::Element> {
        self.pop_back()
    }
    #[inline]
    fn reserve(&mut self, additional: usize) {
        if self.len() + additional > self.capacity() {
            unimplemented!("linked lists are always at max capacity")
        }
    }
}
*/
