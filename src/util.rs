/*
const INLINE_STACK_SIZE: usize = 128;

#[cfg(feature = "smallvec)]
pub(crate) type Stack<T> = smallvec::SmallVec<[T; INLINE_STACK_SIZE]>;
#[cfg(all(feature = "alloc", not(feature = "smallvec")))]
pub(crate) type Stack<T> = alloc::vec::Vec<T>;
#[cfg(all(
    not(feature = "smallvec"),
    not(feature = "alloc"),
))]
pub(crate) type Stack<T> = arrayvec::ArrayVec<[T; INLINE_STACK_SIZE]>;*/

pub trait ArrayMap<T, U> {
    type Output;
    fn array_map(self, f: impl FnMut(T) -> U) -> Self::Output;
    fn array_map_by_ref(&self, f: impl FnMut(&T) -> U) -> Self::Output;
}
impl<T, U> ArrayMap<T, U> for [T; 4] {
    type Output = [U; 4];
    #[inline]
    fn array_map(self, mut f: impl FnMut(T) -> U) -> Self::Output {
        let [
            e0,
            e1,
            e2,
            e3,
        ] = self;
        [
            f(e0),
            f(e1),
            f(e2),
            f(e3),
        ]
    }
    #[inline]
    fn array_map_by_ref(&self, mut f: impl FnMut(&T) -> U) -> Self::Output {
        let [
            e0,
            e1,
            e2,
            e3,
        ] = self;
        [
            f(e0),
            f(e1),
            f(e2),
            f(e3),

        ]
    }
}
impl<T, U> ArrayMap<T, U> for [T; 8] {
    type Output = [U; 8];
    #[inline]
    fn array_map(self, mut f: impl FnMut(T) -> U) -> Self::Output {
        let [
            e0,
            e1,
            e2,
            e3,
            e4,
            e5,
            e6,
            e7,
        ] = self;
        [
            f(e0),
            f(e1),
            f(e2),
            f(e3),
            f(e4),
            f(e5),
            f(e6),
            f(e7),
        ]
    }
    #[inline]
    fn array_map_by_ref(&self, mut f: impl FnMut(&T) -> U) -> Self::Output {
        let [
            e0,
            e1,
            e2,
            e3,
            e4,
            e5,
            e6,
            e7,
        ] = self;
        [
            f(e0),
            f(e1),
            f(e2),
            f(e3),
            f(e4),
            f(e5),
            f(e6),
            f(e7),
        ]
    }
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub unsafe fn unreachable_debugchecked(msg: &str) -> ! {
    #[cfg(debug_assertions)]
    {
        // Most of those panics are in a tree corrupton context, so we should
        // just abort the process to prevent unwinders from collecting corrupted data
        abort_on_panic(|| unreachable!(msg))
    }
    #[cfg(not(debug_assertions))]
    {
        core::hint::unreachable_unchecked()
    }
}

#[inline]
pub fn abort_on_panic<R>(f: impl FnOnce() -> R) -> R {
    #[cfg(feature = "unwind_safety")]
    {
        std::panic::catch_unwind(
            std::panic::AssertUnwindSafe(f)
        ).unwrap_or_else(|_| std::process::exit(101))
    }
    #[cfg(not(feature = "unwind_safety"))]
    {
        f()
    }
}