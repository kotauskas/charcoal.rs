use core::{fmt::Debug, iter::FusedIterator};
use crate::{
    storage::{Storage, DefaultStorage},
    NodeValue,
};
use super::{FreeformTree, Node, NodeData};

// A reference to a node in a freeform tree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree.
#[derive(Debug)]
pub struct NodeRef<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    pub(super) tree: &'a FreeformTree<B, L, K, S>,
    pub(super) key: K,
}
impl<'a, B, L, K, S> NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a new `NodeRef` pointing to the specified key in the storage, or `None` if it's out of bounds.
    pub fn new_raw(tree: &'a FreeformTree<B, L, K, S>, key: K) -> Option<Self> {
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
    pub unsafe fn new_raw_unchecked(tree: &'a FreeformTree<B, L, K, S>, key: K) -> Self {
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
    /// Returns a reference to the sibling of the pointee which comes before it in order, or `None` if it's the first child of its parent.
    pub fn prev_sibling(&self) -> Option<Self> {
        self.node().prev_sibling.as_ref().map(|x| unsafe {
            // SAFETY: prev sibling key is always valid
            Self::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns a reference to the sibling of the pointee which comes after it in order, or `None` if it's the last child of its parent.
    pub fn next_sibling(&self) -> Option<Self> {
        self.node().next_sibling.as_ref().map(|x| unsafe {
            // SAFETY: next sibling key is always valid
            Self::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns a reference to the first child of the node, or `None` if it's a leaf node.
    pub fn first_child(&self) -> Option<Self> {
        if let NodeData::Branch { first_child, .. } = &self.node().value {
            unsafe {
                // SAFETY: first child key is always valid
                Some(Self::new_raw_unchecked(self.tree, first_child.clone()))
            }
        } else {
            None
        }
    }
    /// Returns a reference to the last child of the node, or `None` if it's a leaf node.
    pub fn last_child(&self) -> Option<Self> {
        if let NodeData::Branch { last_child, .. } = &self.node().value {
            unsafe {
                // SAFETY: last child key is always valid
                Some(Self::new_raw_unchecked(self.tree, last_child.clone()))
            }
        } else {
            None
        }
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
    /// Returns `true` if the node is a *branch*, i.e. has one or more child nodes; `false` otherwise.
    pub fn is_branch(&self) -> bool {
        match &self.node().value {
            NodeData::Branch { .. } => true,
            NodeData::Leaf(..) => false,
        }
    }
    /// Returns a reference to the data stored in the node.
    pub fn value(&self) -> NodeValue<&'a B, &'a L> {
        self.node().value.as_ref().into_value()
    }
    /// Returns an iterator over references to the children of the node, or `None` if the node is a leaf node.
    pub fn children(&self) -> Option<NodeChildrenIter<'_, B, L, K, S>> {
        self.first_child().map(Self::siblings)
    }
    /// Returns an iterator over the raw keys of the children of the node, or `None` if the node is a leaf node.
    pub fn children_keys(&self) -> Option<NodeChildKeysIter<'_, B, L, K, S>> {
        self.first_child().map(Self::sibling_keys)
    }
    /// Returns an iterator over references to the siblings of the node. Does not include siblings which come before the current node. The first element yielded is always `self`.
    pub fn siblings(self) -> NodeSiblingsIter<'a, B, L, K, S> {
        NodeSiblingsIter(self.sibling_keys())
    }
    /// Returns an iterator over the raw keys of the siblings of the node. Does not include siblings which come before the current node. The first element yielded is always `self`'s key.
    pub fn sibling_keys(self) -> NodeSiblingKeysIter<'a, B, L, K, S> {
        NodeSiblingKeysIter {
            tree: self.tree,
            key: Some(self.key),
        }
    }

    pub(super) fn node(&self) -> &'a Node<B, L, K> {
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

/// An iterator over keys of the siblings of a freeform tree node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeSiblingKeysIter<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    pub(super) tree: &'a FreeformTree<B, L, K, S>,
    pub(super) key: Option<K>,
}
/// An iterator over keys of the children of a freeform tree node.
pub type NodeChildKeysIter<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>> =
    NodeSiblingKeysIter<'a, B, L, K, S>;
impl<'a, B, L, K, S> Iterator for NodeSiblingKeysIter<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    type Item = K;
    fn next(&mut self) -> Option<Self::Item> {
        let current_key = self.key.take()?;
        let next_key = unsafe {
            // SAFETY: key validity guarantee
            NodeRef::new_raw_unchecked(self.tree, current_key.clone())
                .next_sibling()
                .map(NodeRef::into_raw_key)
        };
        self.key = next_key;
        Some(current_key)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.key.is_some() {
            (1, None)
        } else {
            (0, Some(0))
        }
    }
}
impl<B, L, K, S> FusedIterator for NodeSiblingKeysIter<'_, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
}

/// An iterator over references to the siblings of a freeform tree node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeSiblingsIter<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>(
    pub(super) NodeSiblingKeysIter<'a, B, L, K, S>,
)
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq;
/// An iterator over references to the children of a freeform tree node.
pub type NodeChildrenIter<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>> =
    NodeSiblingsIter<'a, B, L, K, S>;
impl<'a, B, L, K, S> Iterator for NodeSiblingsIter<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    type Item = NodeRef<'a, B, L, K, S>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|key| unsafe {
            // SAFETY: key validity guaranteed
            NodeRef::new_raw_unchecked(self.0.tree, key)
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<B, L, K, S> FusedIterator for NodeSiblingsIter<'_, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
}
