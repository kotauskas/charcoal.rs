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
    iter::FusedIterator,
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
    /// Required to panic if called after a `Stop` value has already been produced. May also panic for other reasons, as appropriate and specified by the documentation on the trait implementation.
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
    /// Required to panic if called after a `Stop` value has already been produced. May also panic for other reasons, as appropriate and specified by the documentation on the trait implementation.
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
    ///
    /// # Panics
    /// Required to panic if the current cursor value is invalid, i.e. it's impossible to determine the previously valid cursor value (as opposed to merely invalid directions but valid cursor, which can be recovered from).
    #[allow(clippy::missing_errors_doc)]
    fn advance_cursor<V>(
        &self,
        cursor: Self::Cursor,
        direction: VisitorDirection<Self::Cursor, V>,
    ) -> CursorResult<Self::Cursor>;
    /// Returns the cursor pointing to the root node.
    fn cursor_to_root(&self) -> Self::Cursor;
    /// Returns a by-reference `NodeValue` of the node at the specified cursor.
    ///
    /// # Panics
    /// Required to panic if the cursor value is invalid.
    fn value_of(&self, cursor: &Self::Cursor) -> NodeValue<&'_ Self::Branch, &'_ Self::Leaf>;
    /// Returns a cursor to the parent of the node at the specified cursor, or `None` if that node is the root node.
    ///
    /// # Panics
    /// Required to panic if the cursor value is invalid. If the cursor is valid but has no parent, the method *must* return `None` instead of panicking.
    fn parent_of(&self, cursor: &Self::Cursor) -> Option<Self::Cursor>;
    /// Returns the number of children of the node at the specified cursor.
    ///
    /// # Panics
    /// Required to panic if the cursor value is invalid.
    fn num_children_of(&self, cursor: &Self::Cursor) -> usize;
    /// Returns a cursor to the *`n`*th child of the node at the specified cursor, or `None` if the child at that index does not exist.
    ///
    /// # Panics
    /// Required to panic if cursor value is invalid.
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor>;

    /// Performs one step of the visitor from the specified cursor, returning either the cursor for the next step or the final result of the visitor if it ended.
    ///
    /// It's a logic error to interleave calls to step through a `Visitor` with equivalent calls for one or more `VisitorMut` on the same traversable. This cannot invoke undefined behavior, but may produce unexpected results, such as infinite loops or panicking.
    ///
    /// # Panics
    /// The visitor itself may panic, but otherwise the method should not add any panics on its own.
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
    /// Whether the `try_remove_branch` and `try_remove_children` methods are implemented. If `false`, `PackedChildren` must be an iterator type which never yields elements, like [`Empty`].
    ///
    /// [`Empty`]: https://doc.rust-lang.org/std/iter/struct.Empty.html " "
    const CAN_PACK_CHILDREN: bool = false;
    /// A container for the leaf children of a branch node.
    type PackedChildren: IntoIterator<Item = Self::Leaf>;

    /// Returns a *mutable* by-reference `NodeValue` of the node at the specified cursor, allowing modifications.
    ///
    /// # Panics
    /// Required to panic if cursor value is invalid.
    fn value_mut_of(
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
    ///
    /// # Panics
    /// Required to panic if cursor value is invalid.
    fn try_remove_leaf<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
    ) -> Result<Self::Leaf, TryRemoveLeafError>;
    /// Attempts to remove a branch node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure (the previous value is passed into the closure). The removed children are put in the specified collector closure in order.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node. The `try_remove_leaf_with` method exists for that.
    /// - The node was the root node, which can never be removed.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    ///
    /// # Panics
    /// Required to panic if cursor value is invalid.
    #[allow(clippy::type_complexity)] // I disagree
    fn try_remove_branch_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
        collector: C,
    ) -> Result<Self::Branch, TryRemoveBranchError>;
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, the value for which is provided by the specified closure. The removed children are put in the specified collector closure in order.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    ///
    /// # Panics
    /// Required to panic if cursor value is invalid.
    #[allow(clippy::type_complexity)] // same here
    fn try_remove_children_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
        collector: C,
    ) -> Result<(), TryRemoveChildrenError>;

    /// Attempts to remove a branch node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure (the previous value is passed into the closure).
    ///
    /// By default, this method is [`unimplemented!`]. In such a case, `try_remove_branch_into` can be used instead. If `CAN_PACK_CHILDREN` is `true`, then it is a logic error to leave it in that state, and the implementor should instead write a proper implementation of this method.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node. The `try_remove_leaf_with` method exists for that.
    /// - The node was the root node, which can never be removed.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    ///
    /// # Panics
    /// Required to panic if cursor value is invalid.
    ///
    /// [`unimplemented!`]: https://doc.rust-lang.org/std/macro.unimplemented.html " "
    #[allow(clippy::type_complexity)] // I disagree
    fn try_remove_branch<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        _cursor: &Self::Cursor,
        _branch_to_leaf: BtL,
    ) -> Result<(Self::Branch, Self::PackedChildren), TryRemoveBranchError> {
        unimplemented!("packing children is not supported by this traversable")
    }
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, the value for which is provided by the specified closure.
    ///
    /// By default, this method is [`unimplemented!`]. In such a case, `try_remove_branch_into` can be used instead. If `CAN_PACK_CHILDREN` is `true`, then it is a logic error to leave it in that state, and the implementor should instead write a proper implementation of this method.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    ///
    /// # Panics
    /// Required to panic if cursor value is invalid.
    ///
    /// [`unimplemented!`]: https://doc.rust-lang.org/std/macro.unimplemented.html " "
    #[allow(clippy::type_complexity)] // same here
    fn try_remove_children<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        _cursor: &Self::Cursor,
        _branch_to_leaf: BtL,
    ) -> Result<Self::PackedChildren, TryRemoveChildrenError> {
        unimplemented!("packing children is not supported by this traversable")
    }

    /// Performs one step of the mutating visitor from the specified cursor, returning either the cursor for the next step or the final result of the visitor if it ended.
    ///
    /// It's a logic error to interleave calls to step through a `VisitorMut` with equivalent calls for another `VisitorMut` or a `Visitor` on the same traversable. This cannot invoke undefined behavior, but may produce unexpected results, such as infinite loops or panicking.
    ///
    /// # Panics
    /// The visitor itself may panic, but otherwise the method should not add any panics on its own.
    fn step_mut<V: VisitorMut>(
        &mut self,
        mut visitor: V,
        cursor: CursorResult<Self::Cursor>,
    ) -> Step<Self::Cursor, V::Output>
    where
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
    fn traverse_mut<V: VisitorMut>(&mut self, visitor: V) -> V::Output
    where
        for<'a> &'a mut Self: BorrowMut<V::Target>,
        Self::Cursor:
            From<<V::Target as Traversable>::Cursor> + Into<<V::Target as Traversable>::Cursor>,
    {
        self.traverse_mut_from(self.cursor_to_root(), visitor)
    }
    /// *Mutably* traverses the traversable from the specified starting point until the end, returning the final result of the visitor.
    fn traverse_mut_from<V: VisitorMut>(
        &mut self,
        starting_cursor: Self::Cursor,
        mut visitor: V,
    ) -> V::Output
    where
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
    pub fn recover(self) -> C {
        self.previous_state
    }
}
impl<C: Clone + Debug + Eq> Display for CursorDirectionError<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad("cannot move cursor in the specified direction")
    }
}
#[cfg(feature = "std")]
#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "std")))]
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
    fn next(&mut self) -> Option<Self::Item> {
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
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        // Not using fully-qualified syntax breaks rust-analyzer because it thinks that I'm using
        // the Iterator::take method which takes one argument
        let cursor =
            Option::take(&mut self.cursor).unwrap_or_else(|| Ok(self.traversable.cursor_to_root()));
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

    fn advance_cursor<V>(
        &self,
        cursor: Self::Cursor,
        direction: VisitorDirection<Self::Cursor, V>,
    ) -> CursorResult<Self::Cursor> {
        (*self).advance_cursor(cursor, direction)
    }
    fn cursor_to_root(&self) -> Self::Cursor {
        (**self).cursor_to_root()
    }
    fn value_of(&self, cursor: &Self::Cursor) -> NodeValue<&'_ Self::Branch, &'_ Self::Leaf> {
        (**self).value_of(cursor)
    }
    fn num_children_of(&self, cursor: &Self::Cursor) -> usize {
        (**self).num_children_of(cursor)
    }
    fn parent_of(&self, cursor: &Self::Cursor) -> Option<Self::Cursor> {
        (**self).parent_of(cursor)
    }
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor> {
        (**self).nth_child_of(cursor, child_num)
    }
}
impl<T: Traversable> Traversable for &mut T {
    type Leaf = T::Leaf;
    type Branch = T::Branch;
    type Cursor = T::Cursor;

    fn advance_cursor<V>(
        &self,
        cursor: Self::Cursor,
        direction: VisitorDirection<Self::Cursor, V>,
    ) -> CursorResult<Self::Cursor> {
        (**self).advance_cursor(cursor, direction)
    }
    fn cursor_to_root(&self) -> Self::Cursor {
        (**self).cursor_to_root()
    }
    fn value_of(&self, cursor: &Self::Cursor) -> NodeValue<&'_ Self::Branch, &'_ Self::Leaf> {
        (**self).value_of(cursor)
    }
    fn num_children_of(&self, cursor: &Self::Cursor) -> usize {
        (**self).num_children_of(cursor)
    }
    fn parent_of(&self, cursor: &Self::Cursor) -> Option<Self::Cursor> {
        (**self).parent_of(cursor)
    }
    fn nth_child_of(&self, cursor: &Self::Cursor, child_num: usize) -> Option<Self::Cursor> {
        (**self).nth_child_of(cursor, child_num)
    }
}
impl<T: Traversable + TraversableMut> TraversableMut for &mut T {
    const CAN_REMOVE_INDIVIDUAL_CHILDREN: bool = T::CAN_REMOVE_INDIVIDUAL_CHILDREN;
    const CAN_PACK_CHILDREN: bool = T::CAN_PACK_CHILDREN;
    type PackedChildren = T::PackedChildren;
    fn value_mut_of(
        &mut self,
        cursor: &Self::Cursor,
    ) -> NodeValue<&'_ mut Self::Branch, &'_ mut Self::Leaf> {
        (*self).value_mut_of(cursor)
    }
    fn try_remove_leaf<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
    ) -> Result<Self::Leaf, TryRemoveLeafError> {
        (*self).try_remove_leaf(cursor, branch_to_leaf)
    }
    #[allow(clippy::type_complexity)]
    fn try_remove_branch_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
        collector: C,
    ) -> Result<Self::Branch, TryRemoveBranchError> {
        (*self).try_remove_branch_into(cursor, branch_to_leaf, collector)
    }
    #[allow(clippy::type_complexity)]
    fn try_remove_children_into<BtL: FnOnce(Self::Branch) -> Self::Leaf, C: FnMut(Self::Leaf)>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
        collector: C,
    ) -> Result<(), TryRemoveChildrenError> {
        (*self).try_remove_children_into(cursor, branch_to_leaf, collector)
    }
    #[allow(clippy::type_complexity)]
    fn try_remove_branch<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
    ) -> Result<(Self::Branch, Self::PackedChildren), TryRemoveBranchError> {
        (*self).try_remove_branch(cursor, branch_to_leaf)
    }
    #[allow(clippy::type_complexity)]
    fn try_remove_children<BtL: FnOnce(Self::Branch) -> Self::Leaf>(
        &mut self,
        cursor: &Self::Cursor,
        branch_to_leaf: BtL,
    ) -> Result<Self::PackedChildren, TryRemoveChildrenError> {
        (*self).try_remove_children(cursor, branch_to_leaf)
    }
}
