use core::{fmt::Debug, borrow::BorrowMut, convert};
use crate::{NodeValue, TryRemoveLeafError, TryRemoveBranchError, TryRemoveChildrenError};
use super::{
    VisitorMut,
    Traversable,
    TraversableMut,
    VisitorDirection,
    CursorResult,
    CursorDirectionError,
};

/// Recursively removes the specified node and all its descendants, using a closure to patch nodes which transition from having one child to having zero children.
///
/// See the [visitor documentation] for the details and performance of the algorithm.
///
/// [visitor documentation]: struct.RecursiveRemovalWith.html " "
#[inline]
pub fn recursively_remove_with<T: TraversableMut>(
    traversable: &mut T,
    cursor: T::Cursor,
    f: impl FnMut(T::Branch) -> T::Leaf,
) -> NodeValue<T::Branch, T::Leaf> {
    let visitor = RecursiveRemovalWith::new(cursor.clone(), f);
    traversable.traverse_mut_from(cursor, visitor)
}
/// Recursively removes the specified node and all its descendants.
///
/// See the [visitor documentation] for the details and performance of the algorithm.
///
/// [visitor documentation]: struct.RecursiveRemovalWith.html " "
#[inline(always)]
pub fn recursively_remove<T>(
    traversable: &mut T,
    cursor: T::Cursor,
) -> NodeValue<T::Branch, T::Leaf>
where
    T: TraversableMut<Branch = <T as Traversable>::Leaf>,
{
    recursively_remove_with(traversable, cursor, convert::identity)
}

/// A `Visitor` which recursively removes a node and all of its descendants, using a closure to patch nodes which transition from having one child to having zero children.
///
/// See also the [`recursively_remove_with`] and [`recursively_remove`] functions, which create and drive the visitor to completion on a traversable.
///
/// # Panics
/// - If the traversable which is being visited incorrectly implements `TraversableMut`, especially `CAN_REMOVE_INDIVIDUAL_CHILDREN` and `parent_of`.
/// - If removing the root node is attempted. If all nodes in a tree need to be removed recursively, it can just be dropped instead.
///
/// # Algorithm details
/// There are two variants of the algorithm, one which runs only if [`CAN_REMOVE_INDIVIDUAL_CHILDREN`] is true and another which runs if it's false.
///
/// Here's how the `CAN_REMOVE_INDIVIDUAL_CHILDREN` algorithm runs:
/// - Start the cursor at the node to remove (*starting the cursor at a different node is a logic error*)
/// - For every traversal step:
///     - If the node at the cursor is a leaf node:
///         - Remove the node immediately
///         - If the node removed was the target node, **end the traversal**, returning the value of the just-removed node
///         - Otherwise, move the cursor to the parent node of the node which has just been removed and **end traversal step, awaiting next iteration**
///     - If the node at the cursor is a branch node:
///         - Try to remove the node together with its children
///             - If it failed because there was a branch child node, move the cursor to that node and **end traversal step, awaiting next iteration**
///             - Otherwise, if the branch node removed was the target node, **end the traversal**, returning the value of the just-removed node
///             - Otherwise, if the target node was among the removed leaf nodes, **end the traversal**, returning the required leaf node
///             - Otherwise, move the cursor to the parent node of the branch node which has just been removed and **end traversal step, awaiting next iteration**
///
/// Here's how the fallback algorithm runs:
/// - Start the cursor at the node to remove (*starting the cursor at a different node is a logic error*)
/// - For every traversal step:
///     - If the node at the cursor is a leaf node, move the cursor to the parent node of the node and **end traversal step, awaiting next iteration**
///     - If the node at the cursor is a branch node:
///         - Try to remove the children of the branch node, turning it into leaf
///             - If it failed because there was a branch child node, move the cursor to that node and **end traversal step, awaiting next iteration**
///             - Otherwise, if the target node was among the removed leaf nodes, **end the traversal**, returning the required leaf node
///             - Otherwise, move the cursor to the parent node of the branch node and **end traversal step, awaiting next iteration**
///
/// [`recursively_remove_with`]: function.recursively_remove_with.html " "
/// [`recursively_remove`]: function.recursively_remove.html " "
/// [`CAN_REMOVE_INDIVIDUAL_CHILDREN`]: ../trait.TraversableMut.html#constant.CAN_REMOVE_INDIVIDUAL_CHILDREN " "
#[derive(Copy, Clone, Debug)]
pub struct RecursiveRemovalWith<T: TraversableMut, F: FnMut(T::Branch) -> T::Leaf> {
    pivot: T::Cursor,
    conversion: F,
}
impl<T: TraversableMut, F: FnMut(T::Branch) -> T::Leaf> RecursiveRemovalWith<T, F> {
    /// Creates the visitor, removing the node at the specified cursor with the specified conversion closure.
    #[inline(always)]
    pub fn new(cursor: T::Cursor, f: F) -> Self {
        Self {
            pivot: cursor,
            conversion: f,
        }
    }
}
/// `RecursiveRemovalWith` which uses a function instead of a closure to patch nodes which transition from having one child to having zero children.
///
/// Use the [`recursively_remove`] function to start this algorithm.
///
/// [`recursively_remove`]: function.recursively_remove.html " "
pub type FnRecursiveRemoval<T> =
    RecursiveRemovalWith<T, fn(<T as Traversable>::Branch) -> <T as Traversable>::Leaf>;
/// See the struct-level documentation for a list of all panicking conditions.
impl<T: TraversableMut, F: FnMut(T::Branch) -> T::Leaf> VisitorMut for RecursiveRemovalWith<T, F> {
    type Target = T;
    type Output = NodeValue<T::Branch, T::Leaf>;

    #[allow(
        clippy::shadow_unrelated, // It's not "unrelated" smh
        clippy::too_many_lines, // I know how to count, thank you very much
    )]
    #[inline]
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
        // Recover from a cursor error. Since we're avoiding incorrect movements, there's no need
        // to expect errors and handle them in a special way.
        let cursor = cursor.unwrap_or_else(CursorDirectionError::recover).into();
        let mut traversable_to_return = traversable;
        let traversable = traversable_to_return.borrow_mut();
        let parent = traversable.parent_of(&cursor);
        let direction = match traversable.value_of(&cursor) {
            NodeValue::Branch(..) if T::CAN_REMOVE_INDIVIDUAL_CHILDREN => {
                let target_child_index = {
                    let mut target = None;
                    let num_children = traversable.num_children_of(&cursor);
                    let get_child = |child| traversable.nth_child_of(&cursor, child);
                    for (i, c) in (0..num_children).filter_map(get_child).enumerate() {
                        if c == self.pivot {
                            target = Some(i);
                            break;
                        }
                    }
                    target
                };
                let result = traversable
                    .try_remove_branch_with(&cursor, &mut self.conversion)
                    .map_err(|e| match e {
                        TryRemoveBranchError::WasRootNode => {
                            panic!("attempted to remove the root node")
                        }
                        TryRemoveBranchError::WasLeafNode => panic!(
                            "\
the node was a branch node but removing it returned TryRemoveBranchError::WasLeafNode"
                        ),
                        TryRemoveBranchError::HadBranchChild(index) => index,
                        TryRemoveBranchError::CannotRemoveIndividualChildren => panic!(
                            "\
CAN_REMOVE_INDIVIDUAL_CHILDREN is true, but removing a branch node returned \
TryRemoveBranchError::CannotRemoveIndividualChildren"
                        ),
                    });
                match result {
                    Ok(val) => {
                        let mut direction = VisitorDirection::SetTo(
                            parent
                                .expect(
                                    "\
the removed node was not a root node but its parent node could not be found",
                                )
                                .into(),
                        );
                        if cursor == self.pivot {
                            direction = VisitorDirection::Stop(NodeValue::Branch(val.0));
                        } else {
                            let child_payload = target_child_index.and_then(|target_child_index| {
                                val.1.into_iter().enumerate().find_map(|(i, c)| {
                                    if i == target_child_index {
                                        Some(c)
                                    } else {
                                        None
                                    }
                                })
                            });
                            if let Some(child_payload) = child_payload {
                                direction = VisitorDirection::Stop(NodeValue::Leaf(child_payload));
                            }
                        }
                        direction
                    }
                    Err(branch_child) => VisitorDirection::Child(branch_child),
                }
            }
            NodeValue::Branch(..) => {
                // We didn't land on the `if T::CAN_REMOVE_INDIVIDUAL_CHILDREN` arm — we have no
                // choice but to seek a branch node with leaf children only, so let's start with
                // the current one
                let target_child_index = {
                    let mut target = None;
                    let num_children = traversable.num_children_of(&cursor);
                    let get_child = |child| traversable.nth_child_of(&cursor, child);
                    for (i, c) in (0..num_children).filter_map(get_child).enumerate() {
                        if c == self.pivot {
                            target = Some(i);
                            break;
                        }
                    }
                    target
                };
                let result = traversable.try_remove_children_with(&cursor, &mut self.conversion);
                match result {
                    Ok(val) => {
                        let mut direction = VisitorDirection::Parent;
                        let child_payload = target_child_index.and_then(|target_child_index| {
                            val.into_iter().enumerate().find_map(|(i, c)| {
                                if i == target_child_index {
                                    Some(c)
                                } else {
                                    None
                                }
                            })
                        });
                        if let Some(child_payload) = child_payload {
                            direction = VisitorDirection::Stop(NodeValue::Leaf(child_payload));
                        }
                        direction
                    }
                    Err(e) => match e {
                        TryRemoveChildrenError::WasLeafNode => panic!(
                            "\
the node was a branch node but removing it returned TryRemoveChildrenError::WasLeafNode"
                        ),
                        TryRemoveChildrenError::HadBranchChild(branch_child) => {
                            VisitorDirection::Child(branch_child)
                        }
                    },
                }
            }
            NodeValue::Leaf(..) if T::CAN_REMOVE_INDIVIDUAL_CHILDREN => {
                let payload = traversable
                    .try_remove_leaf_with(&cursor, &mut self.conversion)
                    .unwrap_or_else(|e| match e {
                        TryRemoveLeafError::WasRootNode => {
                            panic!("attempted to remove the root node")
                        }
                        TryRemoveLeafError::WasBranchNode => panic!(
                            "\
the node was a leaf node but removing it returned TryRemoveLeafError::WasBranchNode"
                        ),
                        TryRemoveLeafError::CannotRemoveIndividualChildren => panic!(
                            "\
CAN_REMOVE_INDIVIDUAL_CHILDREN is true, but removing a leaf node returned \
TryRemoveLeafError::CannotRemoveIndividualChildren"
                        ),
                    });
                if cursor == self.pivot {
                    VisitorDirection::Stop(NodeValue::Leaf(payload))
                } else {
                    VisitorDirection::SetTo(
                        parent
                            .expect(
                                "\
the removed node was not a root node but its parent node could not be found",
                            )
                            .into(),
                    )
                }
            }
            NodeValue::Leaf(..) => {
                // We didn't land on the `if T::CAN_REMOVE_INDIVIDUAL_CHILDREN` arm — we have no
                // choice but to go up to the parent and seek a branch node with leaf children only
                VisitorDirection::SetTo(
                    parent
                        .expect(
                            "\
the removed node was not a root node but its parent node could not be found",
                        )
                        .into(),
                )
            }
        };
        (direction, traversable_to_return)
    }
}
