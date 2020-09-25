use core::{fmt::Debug, borrow::BorrowMut};
use crate::{NodeValue, TryRemoveLeafError, TryRemoveBranchError, TryRemoveChildrenError};
use super::{
    VisitorMut,
    Traversable,
    TraversableMut,
    VisitorDirection,
    CursorResult,
    CursorDirectionError,
};

/// A `Visitor` which recursively removes a node and all of its descendants, using a closure to patch nodes which transition from having one child to having zero children.
#[derive(Copy, Clone, Debug)]
pub struct RecursiveRemovalWith<T: TraversableMut, F: Fn(T::Branch) -> T::Leaf> {
    pivot: T::Cursor,
    conversion: F,
}
impl<T: TraversableMut, F: Fn(T::Branch) -> T::Leaf> VisitorMut for RecursiveRemovalWith<T, F> {
    type Target = T;
    /// If `try_remove_children_with` was used to remove the target node, then `None` is returned. This will be changed in a future version.
    // TODO make this an always-valid value
    type Output = Option<NodeValue<T::Branch, T::Leaf>>;

    #[allow(
        clippy::shadow_unrelated, // It's not "unrelated" smh
        clippy::too_many_lines, // I know how to count, thank you very much
    )]
    // Too many lines? How about:
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
                let result = traversable
                    .try_remove_branch_with(&cursor, &mut self.conversion)
                    .map_or_else(
                        |e| match e {
                            TryRemoveBranchError::WasRootNode => {
                                panic!("attempted to remove the root node")
                            }
                            TryRemoveBranchError::WasLeafNode => panic!(
                                "\
the node was a branch node but removing it returned TryRemoveBranchError::WasLeafNode"
                            ),
                            TryRemoveBranchError::HadBranchChild(index) => Err(index),
                            TryRemoveBranchError::CannotRemoveIndividualChildren => panic!(
                                "\
CAN_REMOVE_INDIVIDUAL_CHILDREN is true, but removing a branch node returned \
TryRemoveBranchError::CannotRemoveIndividualChildren"
                            ),
                        },
                        Ok,
                    );
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
                            direction = VisitorDirection::Stop(
                                Some(NodeValue::Branch(val.0))
                            );
                        } else {
                            let num_children = traversable.num_children_of(&cursor);
                            let get_child = |child| traversable.nth_child_of(&cursor, child);
                            for child_cursor in (0..num_children).filter_map(get_child) {
                                if child_cursor == self.pivot {
                                    direction = VisitorDirection::Stop(
                                        None
                                    );
                                    break;
                                }
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
                let result = traversable.try_remove_children_with(&cursor, &mut self.conversion);
                match result {
                    Ok(val) => {
                    let mut direction = VisitorDirection::Parent;
                    let num_children = traversable.num_children_of(&cursor);
                    let get_child = |child| traversable.nth_child_of(&cursor, child);
                    for child_cursor in (0..num_children).filter_map(get_child) {
                        if child_cursor == self.pivot {
                            direction = VisitorDirection::Stop(None);
                            break;
                        }
                    }
                    direction
                },
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
                    VisitorDirection::Stop(Some(NodeValue::Leaf(payload)))
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
