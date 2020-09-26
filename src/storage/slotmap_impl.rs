use core::fmt::Debug;
use slotmap::{SlotMap, HopSlotMap, DenseSlotMap, Key, Slottable};
use super::Storage;

unsafe impl<K, V> Storage for SlotMap<K, V>
where
    K: Key + Debug + Eq,
    V: Slottable,
{
    type Key = K;
    type Element = V;
    // Those methods clone the keys which have been fed into them — this is perfectly fine, since
    // slotmap keys are actually Copy
    #[inline(always)]
    fn add(&mut self, element: Self::Element) -> Self::Key {
        self.insert(element)
    }
    #[inline(always)]
    fn remove(&mut self, key: &Self::Key) -> Self::Element {
        self.remove(key.clone())
            .expect("the value with this key has already been removed")
    }
    #[inline(always)]
    fn len(&self) -> usize {
        self.len()
    }
    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_key(capacity)
    }
    #[inline(always)]
    unsafe fn get_unchecked(&self, key: &Self::Key) -> &Self::Element {
        self.get_unchecked(key.clone())
    }
    #[inline(always)]
    unsafe fn get_unchecked_mut(&mut self, key: &Self::Key) -> &mut Self::Element {
        self.get_unchecked_mut(key.clone())
    }
    #[inline(always)]
    fn contains_key(&self, key: &Self::Key) -> bool {
        self.contains_key(key.clone())
    }
    #[inline(always)]
    fn get(&self, key: &Self::Key) -> Option<&Self::Element> {
        self.get(key.clone())
    }
    #[inline(always)]
    fn get_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Element> {
        self.get_mut(key.clone())
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
        // FIXME slotmaps don't have a shrink_to_fir method
    }
}

unsafe impl<K, V> Storage for HopSlotMap<K, V>
where
    K: Key + Debug + Eq,
    V: Slottable,
{
    type Key = K;
    type Element = V;
    // Those methods clone the keys which have been fed into them — this is perfectly fine, since
    // slotmap keys are actually Copy
    #[inline(always)]
    fn add(&mut self, element: Self::Element) -> Self::Key {
        self.insert(element)
    }
    #[inline(always)]
    fn remove(&mut self, key: &Self::Key) -> Self::Element {
        self.remove(key.clone())
            .expect("the value with this key has already been removed")
    }
    #[inline(always)]
    fn len(&self) -> usize {
        self.len()
    }
    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_key(capacity)
    }
    #[inline(always)]
    unsafe fn get_unchecked(&self, key: &Self::Key) -> &Self::Element {
        self.get_unchecked(key.clone())
    }
    #[inline(always)]
    unsafe fn get_unchecked_mut(&mut self, key: &Self::Key) -> &mut Self::Element {
        self.get_unchecked_mut(key.clone())
    }
    #[inline(always)]
    fn contains_key(&self, key: &Self::Key) -> bool {
        self.contains_key(key.clone())
    }
    #[inline(always)]
    fn get(&self, key: &Self::Key) -> Option<&Self::Element> {
        self.get(key.clone())
    }
    #[inline(always)]
    fn get_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Element> {
        self.get_mut(key.clone())
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
        // FIXME slotmaps don't have a shrink_to_fir method
    }
}

unsafe impl<K, V> Storage for DenseSlotMap<K, V>
where
    K: Key + Debug + Eq,
    V: Slottable,
{
    type Key = K;
    type Element = V;
    // Those methods clone the keys which have been fed into them — this is perfectly fine, since
    // slotmap keys are actually Copy
    #[inline(always)]
    fn add(&mut self, element: Self::Element) -> Self::Key {
        self.insert(element)
    }
    #[inline(always)]
    fn remove(&mut self, key: &Self::Key) -> Self::Element {
        self.remove(key.clone())
            .expect("the value with this key has already been removed")
    }
    #[inline(always)]
    fn len(&self) -> usize {
        self.len()
    }
    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_key(capacity)
    }
    #[inline(always)]
    unsafe fn get_unchecked(&self, key: &Self::Key) -> &Self::Element {
        self.get_unchecked(key.clone())
    }
    #[inline(always)]
    unsafe fn get_unchecked_mut(&mut self, key: &Self::Key) -> &mut Self::Element {
        self.get_unchecked_mut(key.clone())
    }
    #[inline(always)]
    fn contains_key(&self, key: &Self::Key) -> bool {
        self.contains_key(key.clone())
    }
    #[inline(always)]
    fn get(&self, key: &Self::Key) -> Option<&Self::Element> {
        self.get(key.clone())
    }
    #[inline(always)]
    fn get_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Element> {
        self.get_mut(key.clone())
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
        // FIXME slotmaps don't have a shrink_to_fir method
    }
}
