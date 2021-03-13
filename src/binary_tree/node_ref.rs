use core::fmt::Debug;
use crate::{
    storage::{Storage, DefaultStorage},
    util::unreachable_debugchecked,
    NodeValue,
};
use super::{BinaryTree, Node, NodeData};

/// A reference to a node in a binary tree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree.
#[derive(Debug)]
pub struct NodeRef<'a, B, L, K, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    pub(super) tree: &'a BinaryTree<B, L, K, S>,
    pub(super) key: K,
}
impl<'a, B, L, K, S> NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a new `NodeRef` pointing to the specified key in the storage, or `None` if it's out of bounds.
    pub fn new_raw(tree: &'a BinaryTree<B, L, K, S>, key: K) -> Option<Self> {
        if tree.storage.contains_key(&key) {
            Some(unsafe {
                // SAFETY: we just did a key check
                Self::new_raw_unchecked(tree, key)
            })
        } else {
            None
        }
    }
    /// Creates a new `NodeRef` pointing to the specified key in the storage without doing bounds checking.
    ///
    /// # Safety
    /// Causes *immediate* undefined behavior if the specified key is not present in the storage.
    pub unsafe fn new_raw_unchecked(tree: &'a BinaryTree<B, L, K, S>, key: K) -> Self {
        Self { tree, key }
    }
    /// Returns a reference the raw storage key for the node.
    pub fn raw_key(&self) -> &K {
        &self.key
    }
    /// Consumes the reference and returns the underlying raw storage key for the node.
    pub fn into_raw_key(self) -> K {
        self.key
    }
    /// Returns a reference to the parent node of the pointee, or `None` if it's the root node.
    pub fn parent(&self) -> Option<Self> {
        self.node().parent.as_ref().map(|x| unsafe {
            // SAFETY: nodes can never have out-of-bounds parents
            Self::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns `true` if the node is the root node, `false` otherwise.
    // const_option is not stable, and so are trait bounds on const fn parameters other than Sized
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_root(&self) -> bool {
        self.node().parent.is_none()
    }
    /// Returns `true` if the node is a *leaf*, i.e. does not have child nodes; `false` otherwise.
    pub fn is_leaf(&self) -> bool {
        match &self.node().value {
            NodeData::Branch { .. } => false,
            NodeData::Leaf(..) => true,
        }
    }
    /// Returns `true` if the node is a *branch*, i.e. has one or two child nodes; `false` otherwise.
    pub fn is_branch(&self) -> bool {
        match &self.node().value {
            NodeData::Branch { .. } => true,
            NodeData::Leaf(..) => false,
        }
    }
    /// Returns `true` if the node is a *full branch*, i.e. has exactly two child nodes; `false` otherwise.
    pub fn is_full_branch(&self) -> bool {
        self.children().is_some()
    }
    /// Returns a reference to the data stored in the node.
    pub fn value(&self) -> NodeValue<&'a B, &'a L> {
        self.node().value.as_ref().into_value()
    }
    /// Returns `true` if the node is the left child of its parent, `false` if it's the right one and `None` if it's the root node.
    pub fn is_left_child(&self) -> Option<bool> {
        let parent = self.parent()?;
        let left_child_key = &parent
            .left_child()
            .unwrap_or_else(|| unsafe { unreachable_debugchecked("parent nodes cannot be leaves") })
            .key;
        Some(self.key == *left_child_key)
    }
    /// Returns `true` if the node is the right child of its parent, `false` if it's the left one and `None` if it's the root node.
    pub fn is_right_child(&self) -> Option<bool> {
        let parent = self.parent()?;
        let right_child_key = &parent
            .right_child()
            .unwrap_or_else(|| unsafe { unreachable_debugchecked("parent nodes cannot be leaves") })
            .key;
        Some(self.key == *right_child_key)
    }
    /// Returns references to the children, or `None` if the node is a leaf node or it only has one child. To retreive the left child even if the right one is not present, see `left_child`.
    #[allow(clippy::missing_panics_doc)]
    pub fn children(&self) -> Option<(Self, Self)> {
        match &self.node().value {
            NodeData::Branch {
                left_child,
                right_child,
                ..
            } => right_child
                .as_ref()
                .map(|right_child| (left_child.clone(), right_child.clone())),
            NodeData::Leaf(..) => None,
        }
        .map(|(left_child, right_child)| unsafe {
            // SAFETY: child keys are guaranteed to be valid; a key check to make sure that
            // properly holds is below.
            debug_assert!(
                self.tree.storage.contains_key(&left_child)
                    && self.tree.storage.contains_key(&right_child),
                "\
debug key check failed: tried to reference keys {:?} and {:?} which are not present in the storage",
                &left_child,
                &right_child,
            );
            (
                Self::new_raw_unchecked(self.tree, left_child),
                Self::new_raw_unchecked(self.tree, right_child),
            )
        })
    }
    /// Returns a reference to the left child, or `None` if the node is a leaf node.
    ///
    /// If you need both children, use [`children`] instead.
    ///
    /// [`children`]: #method.children " "
    #[allow(clippy::missing_panics_doc)]
    pub fn left_child(&self) -> Option<Self> {
        if let NodeData::Branch { left_child, .. } = &self.node().value {
            Some(left_child)
        } else {
            None
        }
        .map(|x| unsafe {
            // SAFETY: child keys are guaranteed to be valid; a key check to make sure that
            // properly holds is below.
            debug_assert!(
                self.tree.storage.contains_key(x),
                "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
                x,
            );
            Self::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns a reference to the right child, or `None` if the node is a leaf node.
    ///
    /// If you need both children, use [`children`] instead.
    ///
    /// [`children`]: #method.children " "
    #[allow(clippy::missing_panics_doc)]
    pub fn right_child(&self) -> Option<Self> {
        if let NodeData::Branch { left_child, .. } = &self.node().value {
            Some(left_child.clone())
        } else {
            None
        }
        .map(|x| unsafe {
            // SAFETY: as above
            debug_assert!(
                self.tree.storage.contains_key(&x),
                "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
                &x,
            );
            Self::new_raw_unchecked(self.tree, x)
        })
    }

    fn node(&self) -> &'a Node<B, L, K> {
        debug_assert!(
            self.tree.storage.contains_key(&self.key),
            "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
            &self.key,
        );
        unsafe {
            // SAFETY: all existing NodeRefs are guaranteed to not be dangling
            self.tree.storage.get_unchecked(&self.key)
        }
    }
}
impl<B, L, K, S> Copy for NodeRef<'_, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Copy + Debug + Eq,
{
}
impl<B, L, K, S> Clone for NodeRef<'_, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    fn clone(&self) -> Self {
        Self {
            tree: self.tree,
            key: self.key.clone(),
        }
    }
}
impl<'a, B, L, K, S> From<NodeRef<'a, B, L, K, S>> for NodeValue<&'a B, &'a L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    fn from(op: NodeRef<'a, B, L, K, S>) -> Self {
        op.value()
    }
}
