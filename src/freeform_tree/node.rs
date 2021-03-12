use core::{num::NonZeroIsize, fmt::Debug};
use crate::{
    storage::{ListStorage, MoveFix},
    util::unreachable_debugchecked,
    NodeValue,
};

/// A node of a freeform tree.
///
/// Created by the freeform tree internally and only publicly exposed so that freeform tree storages' generic arguments could be specified.
#[derive(Copy, Clone, Debug, Hash)]
pub struct Node<B, L, K>
where
    K: Clone + Debug + Eq,
{
    pub(super) value: NodeData<B, L, K>,
    pub(super) parent: Option<K>,
    pub(super) prev_sibling: Option<K>,
    pub(super) next_sibling: Option<K>,
}

impl<B, L, K> Node<B, L, K>
where
    K: Clone + Debug + Eq,
{
    #[inline(always)]
    pub(crate) unsafe fn leaf(
        payload: L,
        prev_sibling: Option<K>,
        next_sibling: Option<K>,
        parent: Option<K>,
    ) -> Self {
        Self {
            value: NodeData::Leaf(payload),
            parent,
            prev_sibling,
            next_sibling,
        }
    }
    /*
    Reenable if ever needed
    #[inline(always)]
    pub(crate) unsafe fn branch(
        payload: B,
        first_child: K,
        last_child: K,
        prev_sibling: Option<K>,
        next_sibling: Option<K>,
        parent: Option<K>,
    ) -> Self {
        Self {
            value: NodeData::Branch {
                payload,
                first_child,
                last_child,
            },
            parent,
            prev_sibling,
            next_sibling,
        }
    }
    */
    /// Creates a root node.
    ///
    /// # Safety
    /// The node should not be added into a tree if it already has a root node, as there can only be one.
    #[inline(always)]
    pub(crate) unsafe fn root(value: L) -> Self {
        /*unsafe*/
        {
            // SAFETY: the root node cannot have a parent, therefore
            // finding its parent cannot cause UB as it will just be
            // reported as None
            Self::leaf(value, None, None, None)
        }
    }
}
impl<B, L> MoveFix for Node<B, L, usize> {
    #[inline]
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
            let old_index = i - shifted_by.get() as usize; // undo shift to figure out old index
            Self::fix_move(storage, old_index, i);
        }
    }

    #[inline]
    unsafe fn fix_move<S>(storage: &mut S, previous_index: usize, current_index: usize)
    where
        S: ListStorage<Element = Self>,
    {
        match /*unsafe*/ {
            // SAFETY: index validity is guaranteed for `current_index`.
            &storage.get_unchecked(current_index).value
        } {
            NodeData::Branch { first_child, .. } => {
                let mut current_child = *first_child;
                loop {
                    let child = /*unsafe*/ {
                        // SAFETY: index validity is guaranteed for children.
                        storage.get_unchecked_mut(current_child)
                    };
                    child.parent = Some(current_index);
                    current_child = if let Some(x) = child.next_sibling
                    {x} else {break};
                }
            },
            NodeData::Leaf {..} => {},
        }
        let parent_index = if let Some(index) = /*unsafe*/ {
            // SAFETY: index validity is guaranteed for `current_index`.
            storage.get_unchecked(current_index).parent
        } {
            index
        } else {
            return;
        };
        let parent = storage.get_unchecked_mut(parent_index);
        let first_sibling = {
            match &mut parent.value {
                NodeData::Branch { first_child, .. } => first_child,
                NodeData::Leaf { .. } =>
                /*unsafe*/
                {
                    unreachable_debugchecked("parent nodes cannot be leaves")
                }
            }
        };
        if *first_sibling == previous_index {
            *first_sibling = current_index;
            return;
        }
        let mut current_sibling = *first_sibling;
        loop {
            let node = storage.get_unchecked_mut(current_sibling);
            let next_sibling = &mut node.next_sibling;
            if *next_sibling == Some(previous_index) {
                *next_sibling = Some(current_index);
                return;
            }
            if let Some(next_sibling) = next_sibling {
                current_sibling = *next_sibling;
            } else {
                /*unsafe*/
                {
                    // SAFETY: this mismatch is assumed to never happen as a guarantee
                    // of key validity
                    unreachable_debugchecked("failed to find node in parent's child list")
                }
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
        first_child: K,
        last_child: K,
    },
    Leaf(L),
}
impl<B, L, K> NodeData<B, L, K>
where
    K: Clone + Debug + Eq,
{
    #[inline]
    pub(super) fn as_ref(&self) -> NodeData<&B, &L, K> {
        match self {
            Self::Branch {
                payload,
                first_child,
                last_child,
            } => NodeData::Branch {
                payload,
                first_child: first_child.clone(),
                last_child: last_child.clone(),
            },
            Self::Leaf(x) => NodeData::Leaf(x),
        }
    }
    #[inline]
    pub(super) fn as_mut(&mut self) -> NodeData<&mut B, &mut L, K> {
        match self {
            Self::Branch {
                payload,
                first_child,
                last_child,
            } => NodeData::Branch {
                payload,
                first_child: first_child.clone(),
                last_child: last_child.clone(),
            },
            Self::Leaf(x) => NodeData::Leaf(x),
        }
    }
    #[inline]
    #[allow(clippy::missing_const_for_fn)] // const fn cannot evaluate drop
    pub(super) fn into_value(self) -> NodeValue<B, L> {
        match self {
            Self::Branch { payload, .. } => NodeValue::Branch(payload),
            Self::Leaf(x) => NodeValue::Leaf(x),
        }
    }
}
