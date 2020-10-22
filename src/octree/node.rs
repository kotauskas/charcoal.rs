use core::{
    num::NonZeroIsize,
    fmt::Debug,
};
use crate::{
    storage::{ListStorage, MoveFix},
    NodeValue,
};

/// A node of an octree.
///
/// Created by the octree internally and only publicly exposed so that octree storages' generic arguments could be specified.
#[derive(Copy, Clone, Debug, Hash)]
pub struct Node<B, L, K>
where K: Clone + Debug + Eq,
{
    pub(super) value: NodeData<B, L, K>,
    pub(super) parent: Option<K>,
}
impl<B, L, K> Node<B, L, K>
where K: Clone + Debug + Eq,
{
    #[inline(always)]
    pub(crate) unsafe fn leaf(value: L, parent: Option<K>) -> Self {
        Self {
            value: NodeData::Leaf(value),
            parent,
        }
    }
    /*
    Reenable if ever needed
    #[inline(always)]
    pub(crate) unsafe fn branch(payload: B, children: [K; 8], parent: Option<K>) -> Self {
        Self {
            value: NodeData::Branch {
                payload,
                children,
            },
            parent,
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
            Self::leaf(value, None)
        }
    }
}
impl<B, L> MoveFix for Node<B, L, usize> {
    unsafe fn fix_shift<S>(storage: &mut S, shifted_from: usize, shifted_by: NonZeroIsize)
    where S: ListStorage<Element = Self>,
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

    unsafe fn fix_move<S>(storage: &mut S, previous_index: usize, current_index: usize)
    where S: ListStorage<Element = Self>,
    {
        // SAFETY: index validity is guaranteed for `current_index`.
        if let Some(parent_index) = storage.get_unchecked_mut(current_index).parent {
            let parent = storage.get_unchecked_mut(parent_index);
            match &mut parent.value {
                NodeData::Branch { children, .. } => {
                    for child in children {
                        if *child == previous_index {
                            *child = current_index;
                            return;
                        }
                    }
                    unreachable!("parent's children don't match the old index");
                }
                NodeData::Leaf(..) => unreachable!("unexpected parent leaf node"),
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum NodeData<B, L, K> {
    Branch {
        payload: B,
        children: [K; 8],
    },
    Leaf(L),
}
impl<B, L, K> NodeData<B, L, K>
where K: Clone + Debug + Eq,
{
    #[inline]
    pub(super) fn as_ref(&self) -> NodeData<&B, &L, K> {
        match self {
            Self::Branch {
                payload,
                children,
            } => NodeData::Branch {
                payload,
                children: children.clone(),
            },
            Self::Leaf(x) => NodeData::Leaf(x),
        }
    }
    #[inline]
    pub(super) fn as_mut(&mut self) -> NodeData<&mut B, &mut L, K> {
        match self {
            Self::Branch {
                payload,
                children,
            } => NodeData::Branch {
                payload,
                children: children.clone(),
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