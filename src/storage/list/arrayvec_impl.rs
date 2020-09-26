use arrayvec::{ArrayVec, Array};
use super::ListStorage;

unsafe impl<A> ListStorage for ArrayVec<A>
where A: Array,
{
    type Element = A::Item;

    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        assert_eq!(
            capacity,
            A::CAPACITY,
            "specified capacity does not match the underlying array's size",
        );
        Self::new()
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
        self.as_slice().get_unchecked(index)
    }
    #[inline(always)]
    unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Self::Element {
        self.as_mut_slice().get_unchecked_mut(index)
    }

    #[inline(always)]
    fn get(&self, index: usize) -> Option<&Self::Element> {
        self.as_slice().get(index)
    }
    #[inline(always)]
    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Element> {
        self.as_mut_slice().get_mut(index)
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
        A::CAPACITY
    }
    #[inline(always)]
    fn reserve(&mut self, additional: usize) {
        if self.len() + additional > self.capacity() {
            unimplemented!("ArrayVec does not support allocating memory; if you need such functionality, use SmallVec instead")
        }
    }
    #[inline(always)]
    fn shrink_to_fit(&mut self) {}
    #[inline(always)]
    fn truncate(&mut self, len: usize) {
        self.truncate(len)
    }
}
