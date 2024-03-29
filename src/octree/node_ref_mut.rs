use core::{fmt::Debug, ptr, convert, hint};
use crate::{
    Storage,
    DefaultStorage,
    NodeValue,
    TryRemoveChildrenError,
    MakeBranchError,
    traversal::algorithms,
    util::{ArrayMap, abort_on_panic, unreachable_debugchecked},
};
use super::{Octree, Node, NodeData, PackedChildren, NodeRef};

/// A *mutable* reference to a node in an octree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree and modify it as a whole.
#[derive(Debug)]
pub struct NodeRefMut<'a, B, L, K, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    tree: &'a mut Octree<B, L, K, S>,
    key: K,
}
impl<'a, B, L, K, S> NodeRefMut<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a new `NodeRefMut` pointing to the specified key in the storage, or `None` if it's out of bounds.
    pub fn new_raw(tree: &'a mut Octree<B, L, K, S>, key: K) -> Option<Self> {
        if tree.storage.contains_key(&key) {
            Some(unsafe {
                // SAFETY: we just did a key check
                Self::new_raw_unchecked(tree, key)
            })
        } else {
            None
        }
    }
    /// Creates a new `NodeRefMut` pointing to the specified key in the storage without doing bounds checking.
    ///
    /// # Safety
    /// Causes *immediate* undefined behavior if the specified key is not present in the storage.
    pub unsafe fn new_raw_unchecked(tree: &'a mut Octree<B, L, K, S>, key: K) -> Self {
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
    pub fn parent(&self) -> Option<NodeRef<'_, B, L, K, S>> {
        self.node().parent.as_ref().map(|x| unsafe {
            // SAFETY: nodes can never have out-of-bounds parents
            NodeRef::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns a *mutable* reference to the parent node of the pointee, or `None` if it's the root node.
    pub fn parent_mut(&mut self) -> Option<NodeRefMut<'_, B, L, K, S>> {
        let key = self.node().parent.as_ref().cloned();
        key.map(move |x| unsafe {
            // SAFETY: as above
            Self::new_raw_unchecked(self.tree, x)
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
    pub fn value(&self) -> NodeValue<&'_ B, &'_ L> {
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
    /// Returns a *mutable* reference to the data stored in the node.
    pub fn value_mut(&mut self) -> NodeValue<&'_ mut B, &'_ mut L> {
        self.node_mut().value.as_mut().into_value()
    }
    /// Returns references to the children, or `None` if the node is a leaf node.
    #[allow(clippy::missing_panics_doc)]
    pub fn children(&self) -> Option<[NodeRef<'_, B, L, K, S>; 8]> {
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
            children.array_map_by_ref(|child| /*unsafe*/ {
                // SAFETY: child keys are guaranteed to be valid; a key check to make sure that
                // properly holds is above.
                NodeRef::new_raw_unchecked(self.tree, child.clone())
            })
        })
    }
    /// Returns a reference the `n`-th child, or `None` if the node has no children. Indexing starts from zero, thus the value is in range from 0 to 7.
    ///
    /// # Panics
    /// Will panic if `n > 7`.
    pub fn nth_child(&self, n: u8) -> Option<NodeRef<'_, B, L, K, S>> {
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
            NodeRef::new_raw_unchecked(self.tree, child.clone())
        })
    }
    /// Returns a *mutable* reference the `n`-th child, or `None` if the node has no children. Indexing starts from zero, thus the value is in range from 0 to 7.
    ///
    /// # Panics
    /// Will panic if `n > 7`.
    pub fn nth_child_mut(&mut self, n: u8) -> Option<NodeRefMut<'_, B, L, K, S>> {
        assert!(
            n < 8,
            "\
octrees have either 0 or 8 children, at indicies from 0 to 7, but child at index {} was requested",
            n,
        );
        let children = if let NodeData::Branch { children, .. } = &self.node().value {
            Some(children)
        } else {
            None
        }
        .cloned();
        children.map(move |children| unsafe {
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

    /// Converts a leaf node into a branch node with the specified leaf children, using the provided closure to convert the payload.
    ///
    /// # Errors
    /// Will fail if the node is already a branch node. In such a case, the provided values for the children are returned back to the caller.
    pub fn make_branch_with(
        &mut self,
        children: [L; 8],
        leaf_to_branch: impl FnOnce(L) -> B,
    ) -> Result<(), MakeBranchError<L, PackedChildren<L>>> {
        let old_payload_ref = if let NodeData::Leaf(val) = &self.node().value {
            val
        } else {
            return Err(MakeBranchError {
                packed_children: children.into(),
            });
        };
        let old_payload = unsafe {
            // SAFETY: both pointer validity and overwriting are upheld
            ptr::read(old_payload_ref)
        };
        let payload = leaf_to_branch(old_payload);
        let self_key = self.raw_key().clone();
        let children = children.array_map(|value| {
            self.tree.storage.add(unsafe {
                // SAFETY: key validity of self is implied
                Node::leaf(value, Some(self_key.clone()))
            })
        });
        unsafe {
            // SAFETY: as above
            ptr::write(
                &mut self.node_mut().value,
                NodeData::Branch { children, payload },
            )
        }
        Ok(())
    }

    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, the value for which is provided by the specified closure.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_children_with(
        &mut self,
        branch_to_leaf: impl FnOnce(B) -> L,
    ) -> Result<[L; 8], TryRemoveChildrenError> {
        let children_keys = {
            let children_keys = if let NodeData::Branch { children, .. } = &self.node().value {
                Some(children)
            } else {
                None
            }
            .ok_or(TryRemoveChildrenError::WasLeafNode)?;
            for (c, i) in children_keys.iter().zip(0_u32..) {
                let child_ref = unsafe {
                    // SAFETY: key validity is assumed, since invalid ones cannot possibly be stored
                    self.tree.storage.get_unchecked(c)
                };
                match &child_ref.value {
                    NodeData::Branch { .. } => {
                        return Err(TryRemoveChildrenError::HadBranchChild(i))
                    }
                    NodeData::Leaf(..) => {}
                };
            }
            children_keys.clone() // borrow checker got trolled
        };
        let children_payloads = children_keys.array_map(|key| {
            let node = self.tree.storage.remove(&key);
            match node.value.into_value() {
                NodeValue::Leaf(val) => val,
                NodeValue::Branch(..) => unsafe {
                    // SAFETY: we checked for branch children in the beginning
                    hint::unreachable_unchecked()
                },
            }
        });
        let old_payload_ref = if let NodeData::Branch { payload, .. } = &self.node().value {
            payload
        } else {
            unsafe {
                // SAFETY: we checked for a leaf node in the beginning
                hint::unreachable_unchecked()
            }
        };
        let old_payload = unsafe {
            // SAFETY: we're overwriting the value later, and not using an invalid pointer
            ptr::read(old_payload_ref)
        };
        unsafe {
            // SAFETY: as above
            ptr::write(
                &mut self.node_mut().value,
                NodeData::Leaf(abort_on_panic(|| branch_to_leaf(old_payload))),
            );
        }
        Ok(children_payloads)
    }

    /// Recursively removes the specified node and all its descendants, using a closure to patch nodes which transition from eight to zero children.
    pub fn recursively_remove_with(self, branch_to_leaf: impl FnMut(B) -> L) -> NodeValue<B, L> {
        algorithms::recursively_remove_with(self.tree, self.key, branch_to_leaf)
    }

    fn node(&self) -> &'_ Node<B, L, K> {
        debug_assert!(
            self.tree.storage.contains_key(&self.key),
            "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
            &self.key,
        );
        unsafe {
            // SAFETY: all existing NodeRefMuts are guaranteed to not be dangling
            self.tree.storage.get_unchecked(&self.key)
        }
    }
    fn node_mut(&mut self) -> &'_ mut Node<B, L, K> {
        debug_assert!(
            self.tree.storage.contains_key(&self.key),
            "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
            &self.key,
        );
        unsafe {
            // SAFETY: as above
            self.tree.storage.get_unchecked_mut(&self.key)
        }
    }
}
impl<'a, D, K, S> NodeRefMut<'a, D, D, K, S>
where
    S: Storage<Element = Node<D, D, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Converts a leaf node into a branch node with the specified leaf children, keeping its payload. Because of that, *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail if the node is already a branch node. In such a case, the provided values for the children are returned back to the caller.
    pub fn make_branch(
        &mut self,
        children: [D; 8],
    ) -> Result<(), MakeBranchError<D, PackedChildren<D>>> {
        self.make_branch_with(children, convert::identity)
    }
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, keeping its original payload. Because of that, *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_children(&mut self) -> Result<[D; 8], TryRemoveChildrenError> {
        self.try_remove_children_with(convert::identity)
    }
    /// Recursively removes the specified node and all its descendants. Will keep the original payload of the parent node if removing this node results in a transformation of the parent into a leaf, which is why *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    pub fn recursively_remove(self) -> NodeValue<D> {
        algorithms::recursively_remove(self.tree, self.key)
    }
}

impl<'a, B, L, K, S> From<&'a NodeRefMut<'a, B, L, K, S>> for NodeValue<&'a B, &'a L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    fn from(op: &'a NodeRefMut<'a, B, L, K, S>) -> Self {
        op.value()
    }
}
impl<'a, B, L, K, S> From<&'a mut NodeRefMut<'a, B, L, K, S>> for NodeValue<&'a B, &'a L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    fn from(op: &'a mut NodeRefMut<'a, B, L, K, S>) -> Self {
        op.value()
    }
}

impl<'a, B, L, K, S> From<&'a mut NodeRefMut<'a, B, L, K, S>> for NodeValue<&'a mut B, &'a mut L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    fn from(op: &'a mut NodeRefMut<'a, B, L, K, S>) -> Self {
        op.value_mut()
    }
}

impl<'a, B, L, K, S> From<&'a NodeRefMut<'a, B, L, K, S>> for NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    fn from(op: &'a NodeRefMut<'a, B, L, K, S>) -> Self {
        NodeRef {
            tree: op.tree as &'a _,
            key: op.key.clone(),
        }
    }
}
impl<'a, B, L, K, S> From<&'a mut NodeRefMut<'a, B, L, K, S>> for NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    fn from(op: &'a mut NodeRefMut<'a, B, L, K, S>) -> Self {
        NodeRef {
            tree: op.tree as &'a _,
            key: op.key.clone(),
        }
    }
}
impl<'a, B, L, K, S> From<NodeRefMut<'a, B, L, K, S>> for NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    fn from(op: NodeRefMut<'a, B, L, K, S>) -> Self {
        NodeRef {
            tree: op.tree as &'a _,
            key: op.key,
        }
    }
}
