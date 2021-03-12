use core::{num::NonZeroIsize, fmt::Debug, hint, convert::TryFrom};
use crate::{
    storage::{ListStorage, MoveFix},
    util::unreachable_debugchecked,
    NodeValue,
};

/// A node of a binary tree.
///
/// Created by the binary tree internally and only publicly exposed so that binary tree storages' generic arguments could be specified.
#[derive(Copy, Clone, Debug, Hash)]
pub struct Node<B, L, K>
where
    K: Clone + Debug + Eq,
{
    pub(super) value: NodeData<B, L, K>,
    pub(super) parent: Option<K>,
}
impl<B, L, K> Node<B, L, K>
where
    K: Clone + Debug + Eq,
{
    pub(crate) unsafe fn leaf(value: L, parent: Option<K>) -> Self {
        Self {
            value: NodeData::Leaf(value),
            parent,
        }
    }
    /*
    Reenable if ever needed
        pub(crate) unsafe fn partial_branch(payload: B, child: K, parent: Option<K>) -> Self {
        Self {
            value: NodeData::Branch {
                payload,
                left_child: child,
                right_child: None,
            },
            parent,
        }
    }
        pub(crate) unsafe fn full_branch(payload: B, children: [K; 2], parent: Option<K>) -> Self {
        let [left_child, right_child] = children;
        Self {
            value: NodeData::Branch {
                payload,
                left_child,
                right_child: Some(right_child),
            },
            parent,
        }
    }
    */
    /// Creates a root node.
    ///
    /// # Safety
    /// The node should not be added into a tree if it already has a root node, as there can only be one.
    pub(crate) unsafe fn root(value: L) -> Self {
        /*unsafe*/
        {
            // SAFETY: the root node cannot have a parent, therefore
            // finding its parent cannot cause UB as it will just be
            // reported as None
            Self::leaf(value, None)
        }
    }
}
impl<B, L> MoveFix for Node<B, L, usize> {
    unsafe fn fix_shift<S>(storage: &mut S, shifted_from: usize, shifted_by: NonZeroIsize)
    where
        S: ListStorage<Element = Self>,
    {
        let fix_starting_from = if shifted_by.get() > 0 {
            shifted_from + 1 // If an insertion happened, ignore the new element
        } else {
            shifted_from
        };
        if fix_starting_from >= storage.len() {
            return;
        };
        for i in fix_starting_from..storage.len() {
            let old_index = isize::try_from(i)
                // SAFETY: not having more than isize::MAX elements is an
                // unsafe guarantee of ListStorage
                .unwrap_or_else(|_| hint::unreachable_unchecked())
                - shifted_by.get(); // undo shift to figure out old index
            Self::fix_move(
                storage,
                // SAFETY: same as above
                usize::try_from(old_index).unwrap_or_else(|_| hint::unreachable_unchecked()),
                i,
            );
        }
    }

    unsafe fn fix_move<S>(storage: &mut S, previous_index: usize, current_index: usize)
    where
        S: ListStorage<Element = Self>,
    {
        match /*unsafe*/ {
            // SAFETY: index validity is guaranteed for `current_index`.
            &storage.get_unchecked(current_index).value
        } {
            NodeData::Branch { left_child, right_child, .. } => {
                let (left_child, right_child) = (*left_child, *right_child);
                let mut fix_child = |child| {
                    let child = /*unsafe*/ {
                        // SAFETY: index validity guaranteed for children
                        storage.get_unchecked_mut(child)
                    };
                    child.parent = Some(current_index);
                };
                fix_child(left_child);
                if let Some(right_child) = right_child {
                    fix_child(right_child);
                }
            },
            NodeData::Leaf(..) => {},
        }
        let parent_index = if let Some(i) = /*unsafe*/ {
            // SAFETY: index validity is guaranteed for `current_index`.
            storage.get_unchecked(current_index).parent
        } {
            i
        } else {
            return;
        };
        let parent = storage.get_unchecked_mut(parent_index);
        let (left_child, right_child) = match &mut parent.value {
            NodeData::Branch {
                left_child,
                right_child,
                ..
            } => (left_child, right_child),
            NodeData::Leaf(..) =>
            /*unsafe*/
            {
                unreachable_debugchecked("parent nodes cannot be leaves")
            }
        };
        if *left_child == previous_index {
            *left_child = current_index;
        } else if *right_child == Some(previous_index) {
            *right_child = Some(current_index);
        } else {
            /*unsafe*/
            {
                unreachable_debugchecked(
                    "failed to identify whether the node is the left or right child",
                )
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum NodeData<B, L, K>
where
    K: Clone + Debug + Eq,
{
    Branch {
        payload: B,
        left_child: K,
        right_child: Option<K>,
    },
    Leaf(L),
}
impl<B, L, K> NodeData<B, L, K>
where
    K: Clone + Debug + Eq,
{
    pub(super) fn as_ref(&self) -> NodeData<&B, &L, K> {
        match self {
            Self::Branch {
                payload,
                left_child,
                right_child,
            } => NodeData::Branch {
                payload,
                left_child: (*left_child).clone(),
                right_child: (*right_child).clone(),
            },
            Self::Leaf(x) => NodeData::Leaf(x),
        }
    }
    pub(super) fn as_mut(&mut self) -> NodeData<&mut B, &mut L, K> {
        match self {
            Self::Branch {
                payload,
                left_child,
                right_child,
            } => NodeData::Branch {
                payload,
                left_child: left_child.clone(),
                right_child: right_child.clone(),
            },
            Self::Leaf(x) => NodeData::Leaf(x),
        }
    }
    #[allow(clippy::missing_const_for_fn)] // const fn cannot evaluate drop
    pub(super) fn into_value(self) -> NodeValue<B, L> {
        match self {
            Self::Branch { payload, .. } => NodeValue::Branch(payload),
            Self::Leaf(x) => NodeValue::Leaf(x),
        }
    }
}
