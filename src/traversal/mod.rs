//! Everything related to traversing trees in general.
//!
//! The module is home to the following items:
//! - [`Visitor`] and [`VisitorMut`] — two similar *traits for types which describe algorithms with state*
//! - [`Traversable`] and its optional extension, [`TraversableMut`] — *traits for types which describe tree-like structures* which can be traversed by `Visitor` and `VisitorMut` algorithms
//! - Implementations of ubiquitous algorithms for trees (see the [`algorithms`] module for more)
//! - Niche [`TraverseIter`] and [`TraverseMutIter`] helpers, wrapping a [`Visitor`]/[`Traversable`] or [`VisitorMut`]/[`TraversableMut`] pair into an iterator interface
//! - Helper types: [`Step`], [`VisitorDirection`] and [`CursorDirectionError`]
//!
//! [`algorithms`]: algorithms/index.html " "
//! [`Visitor`]: trait.Visitor.html " "
//! [`VisitorMut`]: trait.VisitorMut.html " "
//! [`Traversable`]: trait.Traversable.html " "
//! [`TraversableMut`]: trait.TraversableMut.html " "
//! [`TraverseIter`]: struct.TraverseIter.html " "
//! [`TraverseMutIter`]: struct.TraverseMutIter.html " "
//! [`Step`]: enum.Step.html " "
//! [`VisitorDirection`]: enum.VisitorDirection.html " "
//! [`CursorDirectionError`]: enum.CursorDirectionError.html " "

pub mod algorithms;

use core::{
    iter::{FusedIterator, FromIterator},
    fmt::{self, Formatter, Debug, Display},
    borrow::{Borrow, BorrowMut},
};
use crate::{NodeValue, TryRemoveLeafError, TryRemoveBranchError, TryRemoveChildrenError};

/// Iterator-like structures which control a traversable tree's cursor and use it to read information from the tree.
///
/// Normal visitors cannot mutate trees without the use of interior mutability. See [`VisitorMut`] for a mutable version of this trait.
///
/// [`VisitorMut`]: trait.VisitorMut.html " "
pub trait Visitor {
    /// The target type which will be traversed by the visitor.
    type Target: Traversable;
    /// The final value produced by the visitor.
    type Output;
    /// Visit the provided node, returning further directions for traversal.
    ///
    /// # Panics
    /// Required to panic if called after a `Stop` value has already been produced.
    fn visit<C>(
        &mut self,
        traversable: impl Borrow<Self::Target>,
        cursor: CursorResult<C>,
    ) -> VisitorDirection<C, Self::Output>
    where
        C: From<<Self::Target as Traversable>::Cursor>
            + Into<<Self::Target as Traversable>::Cursor>
            + Clone
            + Debug
            + Eq;
}
/// A version of [`Visitor`] with an added ability to acquire mutable access to the tree's nodes.
///
/// Mutating visitors require exclusive mutable access to the tree they are visiting. If you only need to read data from the tree instead of mutating it or if the nodes use interior mutability, use [`Visitor`].
///
/// [`Visitor`]: trait.Visitor.html " "
pub trait VisitorMut {
    /// The target type which will be traversed by the visitor.
    type Target: TraversableMut;
    /// The final value produced by the visitor.
    type Output;
    /// Visit the provided node with a mutable reference, returning further directions for traversal and giving back ownership of the mutable borrow to the traversable.
    ///
    /// # Panics
    /// Required to panic if called after a `Stop` value has already been produced.
    fn visit_mut<C, M>(
        &mut self,
        traversable: M,
        cursor: CursorResult<C>,
    ) -> (VisitorDirection<C, Self::Output>, M)
    where
        C: From<<Self::Target as Traversable>::Cursor>
            + Into<<Self::Target as Traversable>::Cursor>
            + Clone
            + Debug
            + Eq,
        M: BorrowMut<Self::Target>;
}
/// The direction in which a visitor wishes to go after visiting a node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum VisitorDirection<C: Clone + Debug + Eq, V> {
    /// Visit the parent of the node which has been visited.
    Parent,
    /// Visit the sibling of the node which has been visited.
    NextSibling,
    /// Visit the `n`-th child of the node which has been visited.
    Child(u32),
    /// Visit a specific cursor.
    ///
    /// Used when the traversable cannot figure out where to go on its own, for example if a visitor removes the node it was on.
    SetTo(C),
    /// Stop the execution of the algorithm, producing a final value.
    Stop(V),
}

/// Data structures which can be traversed using `Visitor`s.
pub trait Traversable: Sized {
    /// The payload of the node if it's a leaf node.
    type Leaf;
    /// The payload of the node if it's a branch node.
    type Branch;
    /// The type for the cursor which will be used for keeping track of the traversed nodes.
    ///
    /// Must be very cheaply clonable, but not required to be `Copy`. Cursors are not guaranteed to be stable — relying on their stability for memory safety might cause undefined behavior. The only case in which those keys should actually be stored for extended periods of time is when a visitor needs to remember node locations, since it's a logic error to interleave `step`/`step_mut` calls for read-only and mutating visitors or for multiple mutating visitors; still, visitors should check for key error conditions and panic if those happen.
    type Cursor: Clone + Debug + Eq;

    /// Advances the specified cursor according to the specified directions from the visitor.
    fn advance_cursor<V>(
        &self,
        cursor: Self::Cursor,
        direction: VisitorDirection<Self::Cursor, V>,
    ) -> CursorResult<Self::Cursor>;
    /// Returns the cursor pointing to the root node.
    fn cursor_to_root(&self) -> Self::Cursor;
    /// Returns a by-reference `NodeValue` of the node at the specified cursor.
    fn value_of(&self, cursor: &Self::Cursor) -> NodeValue<&'_ Self::Branch, &'_ Self::Leaf>;
    /// Returns a cursor to the parent of the node at the specified cursor, or `None` if that node is the root node.
    fn parent_of(&self, cursor: &Self::Cursor) -> Option<Self::Cursor>;
    /// Returns the number of children of the node at the specified cursor.
    fn num_children_of(&self, cursor: &Self::Cursor) -> usize;
    /// Returns a cursor to the *`n`*th child of the node at the specified cursor, or `None` if the child at that index does not exist.
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor>;

    /// Performs one step of the visitor from the specified cursor, returning either the cursor for the next step or the final result of the visitor if it ended.
    ///
    /// It's a logic error to interleave calls to step through a `Visitor` with equivalent calls for one or more `VisitorMut` on the same traversable. This cannot invoke undefined behavior, but may produce unexpected results, such as infinite loops or panicking.
    fn step<V>(
        &self,
        mut visitor: V,
        cursor: CursorResult<Self::Cursor>,
    ) -> Step<Self::Cursor, V::Output>
    where
        V: Visitor,
        for<'a> &'a Self: Borrow<V::Target>,
        Self::Cursor:
            From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
    {
        match visitor.visit(self, cursor.clone()) {
            VisitorDirection::Stop(val) => Step::End(val),
            other => Step::NextCursor(self.advance_cursor(
                match cursor {
                    Ok(val) => val,
                    Err(err) => return Step::NextCursor(Err(err)),
                },
                other,
            )),
        }
    }
    /// Traverses the traversable from the root node until the end, returning the final result of the visitor.
    #[inline(always)]
    fn traverse<V>(&self, visitor: V) -> V::Output
    where
        V: Visitor,
        for<'a> &'a Self: Borrow<V::Target>,
        Self::Cursor:
            From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
    {
        self.traverse_from(self.cursor_to_root(), visitor)
    }
    /// Traverses the traversable from the specified starting point until the end, returning the final result of the visitor.
    fn traverse_from<V>(&self, starting_cursor: Self::Cursor, mut visitor: V) -> V::Output
    where
        V: Visitor,
        for<'a> &'a Self: Borrow<V::Target>,
        Self::Cursor:
            From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
    {
        let mut cursor = Ok(starting_cursor);
        loop {
            match self.step(&mut visitor, cursor.clone()) {
                Step::NextCursor(c) => cursor = c,
                Step::End(f) => return f,
            }
        }
    }
}

/// Data structures which can be traversed using `VisitorMut`s, giving them mutable access to the stored data.
pub trait TraversableMut: Traversable {
    /// Whether the traversable allows removing individual children. This is `true` for trees which have a variable number of children for branches and `false` which don't.
    const CAN_REMOVE_INDIVIDUAL_CHILDREN: bool;
    /// The leaf children of a branch node, packed into an iterable.
    type PackedChildren: IntoIterator<Item = Self::Leaf> + FromIterator<Self::Leaf>;
    /// Returns a *mutable* by-reference `NodeValue` of the node at the specified cursor, allowing modifications.
    fn value_mut_at(
        &mut self,
        cursor: &Self::Cursor,
    ) -> NodeValue<&'_ mut Self::Branch, &'_ mut Self::Leaf>;
    /// Attempts to remove a leaf node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure (the previous value is passed into the closure).
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a branch node, which would require recursion to remove, and this function explicitly does not implement recursive removal.
    /// - The node was the root node, which can never be removed.
    /// - The tree does not allow removing single leaf nodes and only allows removing all children of a branch node.
    fn try_remove_leaf_with<F>(
        &mut self,
        cursor: &Self::Cursor,
        f: F,
    ) -> Result<Self::Leaf, TryRemoveLeafError>
    where
        F: FnOnce(Self::Branch) -> Self::Leaf;
    /// Attempts to remove a branch node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure (the previous value is passed into the closure).
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node. The `try_remove_leaf`/`try_remove_leaf_with` methods exist for that.
    /// - The node was the root node, which can never be removed.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    #[allow(clippy::type_complexity)] // I disagree
    fn try_remove_branch_with<F>(
        &mut self,
        cursor: &Self::Cursor,
        f: F,
    ) -> Result<(Self::Branch, Self::PackedChildren), TryRemoveBranchError>
    where
        F: FnOnce(Self::Branch) -> Self::Leaf;
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, the value for which is provided by the specified closure.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    #[allow(clippy::type_complexity)] // same here
    fn try_remove_children_with<F>(
        &mut self,
        cursor: &Self::Cursor,
        f: F,
    ) -> Result<Self::PackedChildren, TryRemoveChildrenError>
    where
        F: FnOnce(Self::Branch) -> Self::Leaf;

    /// Performs one step of the mutating visitor from the specified cursor, returning either the cursor for the next step or the final result of the visitor if it ended.
    ///
    /// It's a logic error to interleave calls to step through a `VisitorMut` with equivalent calls for another `VisitorMut` or a `Visitor` on the same traversable. This cannot invoke undefined behavior, but may produce unexpected results, such as infinite loops or panicking.
    fn step_mut<V>(
        &mut self,
        mut visitor: V,
        cursor: CursorResult<Self::Cursor>,
    ) -> Step<Self::Cursor, V::Output>
    where
        V: VisitorMut,
        for<'a> &'a mut Self: BorrowMut<V::Target>,
        Self::Cursor:
            From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
    {
        let (directions, borrow) = visitor.visit_mut(self, cursor.clone());
        match directions {
            VisitorDirection::Stop(val) => Step::End(val),
            other => Step::NextCursor(borrow.advance_cursor(
                match cursor {
                    Ok(val) => val,
                    Err(err) => return Step::NextCursor(Err(err)),
                },
                other,
            )),
        }
    }
    /// *Mutably* traverses the traversable from the root node until the end, returning the final result of the visitor.
    #[inline(always)]
    fn traverse_mut<V>(&mut self, visitor: V) -> V::Output
    where
        V: VisitorMut,
        for<'a> &'a mut Self: BorrowMut<V::Target>,
        Self::Cursor:
            From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
    {
        self.traverse_mut_from(self.cursor_to_root(), visitor)
    }
    /// *Mutably* traverses the traversable from the specified starting point until the end, returning the final result of the visitor.
    fn traverse_mut_from<V>(&mut self, starting_cursor: Self::Cursor, mut visitor: V) -> V::Output
    where
        V: VisitorMut,
        for<'a> &'a mut Self: BorrowMut<V::Target>,
        Self::Cursor:
            From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
    {
        let mut cursor = Ok(starting_cursor);
        loop {
            match self.step_mut(&mut visitor, cursor.clone()) {
                Step::NextCursor(c) => cursor = c,
                Step::End(f) => return f,
            }
        }
    }
}

/// The result of a single traversal step.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Step<C: Clone + Debug + Eq, V> {
    /// Traversal is not yet done and another step must be performed at the specified cursor.
    NextCursor(CursorResult<C>),
    /// Traversal has finished with the following final value.
    End(V),
}

/// The error returned by traversables when a visitor gives incorrect directions for the cursor.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct CursorDirectionError<C: Clone + Debug + Eq> {
    /// The last valid state of the cursor, right before an incorrect movement was attempted.
    pub previous_state: C,
}
/// A result type for functions receiving or returning a cursor which has possibly been incorrectly driven.
pub type CursorResult<C> = Result<C, CursorDirectionError<C>>;
impl<C: Clone + Debug + Eq> CursorDirectionError<C> {
    /// Returns the previous state of the cursor.
    ///
    /// Primarily used as a convenience function for `unwrap_or_else` on `CursorResult`.
    #[inline(always)]
    pub fn recover(self) -> C {
        self.previous_state
    }
}
impl<C: Clone + Debug + Eq> Display for CursorDirectionError<C> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("cannot move cursor in the specified direction")
    }
}
#[cfg(feature = "std")]
impl<C: Clone + Debug + Eq> std::error::Error for CursorDirectionError<C> {}

/// An iterator which groups a [`Traversable`] and a [`Visitor`], performing one step with each iteration.
///
/// The iterator produces values of type `Option<V::Output>`, returning `Some(None)` when calling `next` if the visitor did not stop yet and `Some(Some(...))` when it has produced a final value. After that, it will only return `None`.
///
/// See [`TraverseMutIter`] for a version which uses [`TraversableMut`] and [`VisitorMut`] instead.
///
/// [`Visitor`]: trait.Visitor.html " "
/// [`VisitorMut`]: trait.VisitorMut.html " "
/// [`Traversable`]: trait.Traversable.html " "
/// [`TraversableMut`]: trait.TraversableMut.html " "
/// [`TraverseMutIter`]: struct.TraverseMutIter.html " "
pub struct TraverseIter<V, T>
where
    V: Visitor,
    T: Traversable,
    for<'a> &'a T: Borrow<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
    visitor: V,
    traversable: T,
    cursor: Option<Result<T::Cursor, CursorDirectionError<T::Cursor>>>,
    finished: bool,
}
impl<V, T> TraverseIter<V, T>
where
    V: Visitor,
    T: Traversable,
    for<'a> &'a T: Borrow<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
    /// Creates a traversal iterator with the specified traversable and visitor.
    #[inline(always)]
    pub fn new(visitor: V, traversable: T) -> Self {
        Self {
            visitor,
            traversable,
            cursor: None,
            finished: false,
        }
    }
}
impl<V, T> From<(V, T)> for TraverseIter<V, T>
where
    V: Visitor,
    T: Traversable,
    for<'a> &'a T: Borrow<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
    #[inline(always)]
    fn from(op: (V, T)) -> Self {
        Self::new(op.0, op.1)
    }
}
impl<V, T> Iterator for TraverseIter<V, T>
where
    V: Visitor,
    T: Traversable,
    for<'a> &'a T: Borrow<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
    type Item = Option<V::Output>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        #[cold] // FusedIterator moment
        if self.finished {
            return None;
        }
        let cursor = self
            .cursor
            .take()
            .unwrap_or_else(|| Ok(self.traversable.cursor_to_root()));
        match self.traversable.step(&mut self.visitor, cursor) {
            Step::NextCursor(c) => {
                self.cursor = Some(c);
                Some(None)
            }
            Step::End(f) => {
                self.finished = true;
                Some(Some(f))
            }
        }
    }
}
impl<V, T> Debug for TraverseIter<V, T>
where
    V: Visitor + Debug,
    T: Traversable + Debug,
    for<'a> &'a T: Borrow<V::Target>,
    T::Cursor:
        From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor> + Debug,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TraverseIter")
            .field("visitor", &self.visitor)
            .field("traversabe", &self.traversable)
            .field("cursor", &self.cursor)
            .field("finished", &self.finished)
            .finish()
    }
}
impl<V, T> FusedIterator for TraverseIter<V, T>
where
    V: Visitor,
    T: Traversable,
    for<'a> &'a T: Borrow<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
}

/// An iterator which groups a [`TraversableMut`] and a [`VisitorMut`], performing one step with each iteration.
///
/// The iterator produces values of type `Option<V::Output>`, returning `Some(None)` when calling `next` if the visitor did not stop yet and `Some(Some(...))` when it has produced a final value. After that, it will only return `None`.
///
/// See [`TraverseIter`] for a version which uses [`Traversable`] and [`Visitor`] instead.
///
/// [`Visitor`]: trait.Visitor.html " "
/// [`VisitorMut`]: trait.VisitorMut.html " "
/// [`Traversable`]: trait.Traversable.html " "
/// [`TraversableMut`]: trait.TraversableMut.html " "
/// [`TraverseIter`]: struct.TraverseIter.html " "
pub struct TraverseMutIter<V, T>
where
    V: VisitorMut,
    T: TraversableMut,
    for<'a> &'a mut T: BorrowMut<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
    visitor: V,
    traversable: T,
    cursor: Option<CursorResult<T::Cursor>>,
    finished: bool,
}
impl<V, T> TraverseMutIter<V, T>
where
    V: VisitorMut,
    T: TraversableMut,
    for<'a> &'a mut T: BorrowMut<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
    /// Creates a mutating traversal iterator with the specified traversable and visitor.
    #[inline(always)]
    pub fn new(visitor: V, traversable: T) -> Self {
        Self {
            visitor,
            traversable,
            cursor: None,
            finished: false,
        }
    }
}
impl<V, T> From<(V, T)> for TraverseMutIter<V, T>
where
    V: VisitorMut,
    T: TraversableMut,
    for<'a> &'a mut T: BorrowMut<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
    #[inline(always)]
    fn from(op: (V, T)) -> Self {
        Self::new(op.0, op.1)
    }
}
impl<V, T> Iterator for TraverseMutIter<V, T>
where
    V: VisitorMut,
    T: TraversableMut,
    for<'a> &'a mut T: BorrowMut<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
    type Item = Option<V::Output>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        #[cold] // FusedIterator moment
        if self.finished {
            return None;
        }
        // Not using fully-qualified syntax breaks rust-analyzer because it thinks that I'm using
        // the Iterator::take method which takes one argument
        let cursor = Option::take(&mut self.cursor)
            .unwrap_or_else(|| Ok(self.traversable.cursor_to_root()));
        match self.traversable.step_mut(&mut self.visitor, cursor) {
            Step::NextCursor(c) => {
                self.cursor = Some(c);
                Some(None)
            }
            Step::End(f) => {
                self.finished = true;
                Some(Some(f))
            }
        }
    }
}
impl<V, T> Debug for TraverseMutIter<V, T>
where
    V: VisitorMut + Debug,
    T: TraversableMut + Debug,
    for<'a> &'a mut T: BorrowMut<V::Target>,
    T::Cursor:
        From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor> + Debug,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TraverseMutIter")
            .field("visitor", &self.visitor)
            .field("traversabe", &self.traversable)
            .field("cursor", &self.cursor)
            .field("finished", &self.finished)
            .finish()
    }
}
impl<V, T> FusedIterator for TraverseMutIter<V, T>
where
    V: VisitorMut,
    T: TraversableMut,
    for<'a> &'a mut T: BorrowMut<V::Target>,
    T::Cursor: From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
{
}

//───────────────────────────────────────────────────────────────────────┐
// Implementations for pointer types and other standard library storages │
//───────────────────────────────────────────────────────────────────────┘
impl<T: Visitor> Visitor for &mut T {
    type Target = T::Target;
    type Output = T::Output;
    #[inline(always)]
    fn visit<C>(
        &mut self,
        traversable: impl Borrow<Self::Target>,
        cursor: CursorResult<C>,
    ) -> VisitorDirection<C, Self::Output>
    where
        C: From<<Self::Target as Traversable>::Cursor>
            + Into<<Self::Target as Traversable>::Cursor>
            + Clone
            + Debug
            + Eq,
    {
        (*self).visit(traversable, cursor)
    }
}
impl<T: VisitorMut> VisitorMut for &mut T {
    type Target = T::Target;
    type Output = T::Output;
    #[inline(always)]
    fn visit_mut<C, M>(
        &mut self,
        traversable: M,
        cursor: CursorResult<C>,
    ) -> (VisitorDirection<C, Self::Output>, M)
    where
        C: From<<Self::Target as Traversable>::Cursor>
            + Into<<Self::Target as Traversable>::Cursor>
            + Clone
            + Debug
            + Eq,
        M: BorrowMut<Self::Target>,
    {
        (*self).visit_mut(traversable, cursor)
    }
}
impl<T: Traversable> Traversable for &T {
    type Leaf = T::Leaf;
    type Branch = T::Branch;
    type Cursor = T::Cursor;

    #[inline(always)]
    fn advance_cursor<V>(
        &self,
        cursor: Self::Cursor,
        direction: VisitorDirection<Self::Cursor, V>,
    ) -> CursorResult<Self::Cursor> {
        (*self).advance_cursor(cursor, direction)
    }
    #[inline(always)]
    fn cursor_to_root(&self) -> Self::Cursor {
        (**self).cursor_to_root()
    }
    #[inline(always)]
    fn value_of(&self, cursor: &Self::Cursor) -> NodeValue<&'_ Self::Branch, &'_ Self::Leaf> {
        (**self).value_of(cursor)
    }
    #[inline(always)]
    fn num_children_of(&self, cursor: &Self::Cursor) -> usize {
        (**self).num_children_of(cursor)
    }
    #[inline(always)]
    fn parent_of(&self, cursor: &Self::Cursor) -> Option<Self::Cursor> {
        (**self).parent_of(cursor)
    }
    #[inline(always)]
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor> {
        (**self).nth_child_of(cursor, child_num)
    }
}
impl<T: Traversable> Traversable for &mut T {
    type Leaf = T::Leaf;
    type Branch = T::Branch;
    type Cursor = T::Cursor;

    #[inline(always)]
    fn advance_cursor<V>(
        &self,
        cursor: Self::Cursor,
        direction: VisitorDirection<Self::Cursor, V>,
    ) -> CursorResult<Self::Cursor> {
        (**self).advance_cursor(cursor, direction)
    }
    #[inline(always)]
    fn cursor_to_root(&self) -> Self::Cursor {
        (**self).cursor_to_root()
    }
    #[inline(always)]
    fn value_of(&self, cursor: &Self::Cursor) -> NodeValue<&'_ Self::Branch, &'_ Self::Leaf> {
        (**self).value_of(cursor)
    }
    #[inline(always)]
    fn num_children_of(&self, cursor: &Self::Cursor) -> usize {
        (**self).num_children_of(cursor)
    }
    #[inline(always)]
    fn parent_of(&self, cursor: &Self::Cursor) -> Option<Self::Cursor> {
        (**self).parent_of(cursor)
    }
    #[inline(always)]
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor> {
        (**self).nth_child_of(cursor, child_num)
    }
}
impl<T: Traversable + TraversableMut> TraversableMut for &mut T {
    const CAN_REMOVE_INDIVIDUAL_CHILDREN: bool = T::CAN_REMOVE_INDIVIDUAL_CHILDREN;
    type PackedChildren = T::PackedChildren;
    #[inline(always)]
    fn value_mut_at(
        &mut self,
        cursor: &Self::Cursor,
    ) -> NodeValue<&'_ mut Self::Branch, &'_ mut Self::Leaf> {
        (*self).value_mut_at(cursor)
    }
    #[inline(always)]
    fn try_remove_leaf_with<F>(
        &mut self,
        cursor: &Self::Cursor,
        f: F,
    ) -> Result<Self::Leaf, TryRemoveLeafError>
    where
        F: FnOnce(Self::Branch) -> Self::Leaf,
    {
        (*self).try_remove_leaf_with(cursor, f)
    }
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    fn try_remove_branch_with<F>(
        &mut self,
        cursor: &Self::Cursor,
        f: F,
    ) -> Result<(Self::Branch, Self::PackedChildren), TryRemoveBranchError>
    where
        F: FnOnce(Self::Branch) -> Self::Leaf,
    {
        (*self).try_remove_branch_with(cursor, f)
    }
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    fn try_remove_children_with<F>(
        &mut self,
        cursor: &Self::Cursor,
        f: F,
    ) -> Result<Self::PackedChildren, TryRemoveChildrenError>
    where
        F: FnOnce(Self::Branch) -> Self::Leaf,
    {
        (*self).try_remove_children_with(cursor, f)
    }
}
