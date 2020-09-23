use core::borrow::BorrowMut;
use crate::NodeValue;
use super::{
    Visitor, VisitorMut,
    Traversable, TraversableMut,
    Step, VisitorDirection,
    CursorDirectionError,
};

/// A `Visitor` which recursively removes a node and all of its descendants.
pub struct RecursiveRemoval<T: TraversableMut> {
    pivot: T::Cursor,
    // TODO
}
impl<T: TraversableMut> VisitorMut for RecursiveRemoval<T> {
    type Target = T;
    type Output = NodeValue<T::Branch, T::Leaf>;

    fn visit_mut<C, M>(
        &mut self,
        traversable: M,
        cursor: Result<C, CursorDirectionError>,
    ) -> (VisitorDirection<Self::Output>, M)
    where
        C: Into<T::Cursor>,
        M: BorrowMut<Self::Target> {
        todo!()
    }
}