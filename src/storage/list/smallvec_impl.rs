use smallvec::{SmallVec, Array};
use super::ListStorage;

unsafe impl<A> ListStorage for SmallVec<A>
where A: Array,
{
    type Element = A::Item;

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
