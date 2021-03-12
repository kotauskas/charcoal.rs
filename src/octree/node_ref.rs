use core::fmt::Debug;
use super::{Octree, Node, NodeData};
use crate::{Storage, DefaultStorage, NodeValue, util::unreachable_debugchecked};

/// A reference to a node in an octree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree.
#[derive(Debug)]
pub struct NodeRef<'a, B, L, K, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    pub(super) tree: &'a Octree<B, L, K, S>,
    pub(super) key: K,
}
impl<'a, B, L, K, S> NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a new `NodeRef` pointing to the specified key in the storage, or `None` if it's out of bounds.
    pub fn new_raw(tree: &'a Octree<B, L, K, S>, key: K) -> Option<Self> {
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
    pub unsafe fn new_raw_unchecked(tree: &'a Octree<B, L, K, S>, key: K) -> Self {
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
    /// Returns `true` if the node is a *branch*, i.e. has child nodes; `false` otherwise.
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
    /// Returns the index of the child among its siblings, or `None` if it's the root node.
    pub fn child_index(&self) -> Option<u8> {
        let parent = self.parent()?;
        for (sibling, index) in parent
            .children()
            .unwrap_or_else(|| unsafe { unreachable_debugchecked("parent nodes cannot be leaves") })
            .iter()
            .zip(0_u8..)
        {
            if sibling.key == self.key {
                return Some(index);
            }
        }
        unsafe { unreachable_debugchecked("failed to find node in parent's child list") }
    }
    /// Returns references to the children, or `None` if the node is a leaf node.
    pub fn children(&self) -> Option<[Self; 8]> {
        if let NodeData::Branch { children, .. } = &self.node().value {
            Some(children)
        } else {
            None
        }
        .map(|children| unsafe {
            for c in children {
                debug_assert!(
                    self.tree.storage.contains_key(c),
                    "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
                    c,
                );
            }
            let [child_0, child_1, child_2, child_3, child_4, child_5, child_6, child_7] =
                children.clone();
            // There might be a way to make this look nicer.
            [
                // SAFETY: child keys are guaranteed to be valid; a key check to make sure that
                // properly holds is above.
                Self::new_raw_unchecked(self.tree, child_0),
                Self::new_raw_unchecked(self.tree, child_1),
                Self::new_raw_unchecked(self.tree, child_2),
                Self::new_raw_unchecked(self.tree, child_3),
                Self::new_raw_unchecked(self.tree, child_4),
                Self::new_raw_unchecked(self.tree, child_5),
                Self::new_raw_unchecked(self.tree, child_6),
                Self::new_raw_unchecked(self.tree, child_7),
            ]
        })
    }
    /// Returns a reference to the `n`-th child, or `None` if the node has no children. Indexing starts from zero, thus the value is in range from 0 to 7.
    ///
    /// # Panics
    /// Will panic if `n > 7`.
    pub fn nth_child(&self, n: u8) -> Option<Self> {
        assert!(
            n < 8,
            "\
octrees have either 0 or 8 children, at indicies from 0 to 7, but child at index {} was requested",
            n,
        );
        if let NodeData::Branch { children, .. } = &self.node().value {
            Some(children)
        } else {
            None
        }
        .map(|children| unsafe {
            // SAFETY: the beginning of the function checks n
            let child = children.get_unchecked(n as usize);

            // SAFETY: child keys are guaranteed to be valid; a key check to make sure that
            // properly holds is below.
            debug_assert!(
                self.tree.storage.contains_key(child),
                "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
                child,
            );
            Self::new_raw_unchecked(self.tree, child.clone())
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
