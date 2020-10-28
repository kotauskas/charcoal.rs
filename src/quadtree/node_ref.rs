use core::{
    fmt::Debug,
    ptr,
    convert,
    hint,
};
use super::{Quadtree, Node, NodeData, PackedChildren};
use crate::{
    Storage,
    DefaultStorage,
    NodeValue,
    TryRemoveChildrenError,
    MakeBranchError,
    traversal::algorithms,
    util::ArrayMap,
};

/// A reference to a node in a quadtree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree.
#[derive(Debug)]
pub struct NodeRef<'a, B, L, K, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    tree: &'a Quadtree<B, L, K, S>,
    key: K,
}
impl<'a, B, L, K, S> NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a new `NodeRef` pointing to the specified key in the storage, or `None` if it's out of bounds.
    #[inline]
    pub fn new_raw(tree: &'a Quadtree<B, L, K, S>, key: K) -> Option<Self> {
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
    #[inline(always)]
    pub unsafe fn new_raw_unchecked(tree: &'a Quadtree<B, L, K, S>, key: K) -> Self {
        Self { tree, key }
    }
    /// Returns a reference the raw storage key for the node.
    #[inline(always)]
    pub fn raw_key(&self) -> &K {
        &self.key
    }
    /// Consumes the reference and returns the underlying raw storage key for the node.
    #[inline(always)]
    pub fn into_raw_key(self) -> K {
        self.key
    }
    /// Returns a reference to the parent node of the pointee, or `None` if it's the root node.
    #[inline]
    pub fn parent(&self) -> Option<Self> {
        self.node().parent.as_ref().map(|x| unsafe {
            // SAFETY: nodes can never have out-of-bounds parents
            Self::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns `true` if the node is the root node, `false` otherwise.
    #[inline(always)]
    // trait bounds on const fn parameters other than Sized are not stable
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_root(&self) -> bool {
        self.node().parent.is_none()
    }
    /// Returns `true` if the node is a *leaf*, i.e. does not have child nodes; `false` otherwise.
    #[inline]
    pub fn is_leaf(&self) -> bool {
        match &self.node().value {
            NodeData::Branch {..} => false,
            NodeData::Leaf(..) => true,
        }
    }
    /// Returns `true` if the node is a *branch*, i.e. has child nodes; `false` otherwise.
    #[inline]
    pub fn is_branch(&self) -> bool {
        match &self.node().value {
            NodeData::Branch {..} => true,
            NodeData::Leaf(..) => false,
        }
    }
    /// Returns a reference to the data stored in the node.
    #[inline(always)]
    pub fn value(&self) -> NodeValue<&'a B, &'a L> {
        self.node().value.as_ref().into_value()
    }

    /// Returns references to the children, or `None` if the node is a leaf node.
    #[inline]
    pub fn children(&self) -> Option<[Self; 4]> {
        match &self.node().value {
            NodeData::Branch { children, .. } => Some(children),
            NodeData::Leaf(..) => None,
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
            let [
                child_0, child_1, child_2, child_3,
            ] = children.clone();
            // There might be a way to make this look nicer.
            [
                // SAFETY: child keys are guaranteed to be valid; a key check to make sure that
                // properly holds is above.
                Self::new_raw_unchecked(self.tree, child_0),
                Self::new_raw_unchecked(self.tree, child_1),
                Self::new_raw_unchecked(self.tree, child_2),
                Self::new_raw_unchecked(self.tree, child_3),
            ]
        })
    }
    /// Returns a reference to the `n`-th child, or `None` if the node has no children. Indexing starts from zero, thus the value is in range from 0 to 3.
    ///
    /// # Panics
    /// Will panic if `n > 3`.
    #[inline]
    pub fn nth_child(&self, n: u8) -> Option<Self> {
        assert!(
            n < 4,
            "\
quadtrees have either 0 or 4 children, at indicies \
from 0 to 3, but child at index {} was requested",
            n,
        );
        match &self.node().value {
            NodeData::Branch { children, .. } => Some(children),
            NodeData::Leaf(_) => None,
        }.map(|children| unsafe {
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

    #[inline(always)]
    fn node(&self) -> &'a Node<B, L, K> {
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
{}
impl<B, L, K, S> Clone for NodeRef<'_, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    #[inline(always)]
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
    #[inline(always)]
    fn from(op: NodeRef<'a, B, L, K, S>) -> Self {
        op.value()
    }
}

/// A *mutable* reference to a node in a quadtree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree and modify it as a whole.
#[derive(Debug)]
pub struct NodeRefMut<'a, B, L, K, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    tree: &'a mut Quadtree<B, L, K, S>,
    key: K,
}
impl<'a, B, L, K, S> NodeRefMut<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a new `NodeRefMut` pointing to the specified key in the storage, or `None` if it's out of bounds.
    #[inline]
    pub fn new_raw(tree: &'a mut Quadtree<B, L, K, S>, key: K) -> Option<Self> {
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
    #[inline(always)]
    pub unsafe fn new_raw_unchecked(tree: &'a mut Quadtree<B, L, K, S>, key: K) -> Self {
        Self { tree, key }
    }
    /// Returns a reference the raw storage key for the node.
    #[inline(always)]
    pub fn raw_key(&self) -> &K {
        &self.key
    }
    /// Consumes the reference and returns the underlying raw storage key for the node.
    #[inline(always)]
    pub fn into_raw_key(self) -> K {
        self.key
    }
    /// Returns a reference to the parent node of the pointee, or `None` if it's the root node.
    #[inline]
    pub fn parent(&'_ self) -> Option<NodeRef<'_, B, L, K, S>> {
        self.node().parent.as_ref().map(|x| unsafe {
            // SAFETY: nodes can never have out-of-bounds parents
            NodeRef::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns a *mutable* reference to the parent node of the pointee, or `None` if it's the root node.
    #[inline]
    pub fn parent_mut(&'_ mut self) -> Option<NodeRefMut<'_, B, L, K, S>> {
        let key = self.node().parent.as_ref().cloned();
        key.map(move |x| unsafe {
            // SAFETY: as above
            Self::new_raw_unchecked(self.tree, x)
        })
    }
    /// Returns `true` if the node is the root node, `false` otherwise.
    #[inline(always)]
    // const_option is not stable, and so are trait bounds on const fn parameters other than Sized
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_root(&self) -> bool {
        self.node().parent.is_none()
    }
    /// Returns `true` if the node is a *leaf*, i.e. does not have child nodes; `false` otherwise.
    #[inline]
    pub fn is_leaf(&self) -> bool {
        match &self.node().value {
            NodeData::Branch {..} => false,
            NodeData::Leaf(..) => true,
        }
    }
    /// Returns `true` if the node is a *branch*, i.e. has child nodes; `false` otherwise.
    #[inline]
    pub fn is_branch(&self) -> bool {
        match &self.node().value {
            NodeData::Branch {..} => true,
            NodeData::Leaf(..) => false,
        }
    }
    /// Returns a reference to the data stored in the node.
    #[inline(always)]
    pub fn value(&self) -> NodeValue<&'_ B, &'_ L> {
        self.node().value.as_ref().into_value()
    }
    /// Returns a *mutable* reference to the data stored in the node.
    #[inline(always)]
    pub fn value_mut(&mut self) -> NodeValue<&'_ mut B, &'_ mut L> {
        self.node_mut().value.as_mut().into_value()
    }
    /// Returns references to the children, or `None` if the node is a leaf node.
    #[inline]
    pub fn children(&self) -> Option<[NodeRef<'_, B, L, K, S>; 4]> {
        match &self.node().value {
            NodeData::Branch { children, .. } => Some(children),
            NodeData::Leaf(..) => None,
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
            let [
                child_0, child_1, child_2, child_3,
            ] = children.clone();
            // There might be a way to make this look nicer.
            [
                // SAFETY: child keys are guaranteed to be valid; a key check to make sure that
                // properly holds is above.
                NodeRef::new_raw_unchecked(self.tree, child_0),
                NodeRef::new_raw_unchecked(self.tree, child_1),
                NodeRef::new_raw_unchecked(self.tree, child_2),
                NodeRef::new_raw_unchecked(self.tree, child_3),
            ]
        })
    }
    /// Returns a reference the `n`-th child, or `None` if the node has no children. Indexing starts from zero, thus the value is in range from 0 to 7.
    ///
    /// # Panics
    /// Will panic if `n > 3`.
    #[inline]
    pub fn nth_child(&self, n: u8) -> Option<NodeRef<'_, B, L, K, S>> {
        assert!(
            n < 4,
            "\
quadtrees have either 0 or 4 children, at indicies \
from 0 to 3, but child at index {} was requested",
            n,
        );
        match &self.node().value {
            NodeData::Branch { children, .. } => Some(children),
            NodeData::Leaf(_) => None,
        }.map(|children| unsafe {
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
    /// Will panic if `n > 3`.
    #[inline]
    pub fn nth_child_mut(&mut self, n: u8) -> Option<NodeRefMut<'_, B, L, K, S>> {
        assert!(
            n < 4,
            "\
quadtrees have either 0 or 4 children, at indicies \
from 0 to 3, but child at index {} was requested",
            n,
        );
        let children = match &self.node().value {
            NodeData::Branch { children, .. } => Some(children),
            NodeData::Leaf(_) => None,
        }.cloned();
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
        children: [L; 4],
        f: impl FnOnce(L) -> B,
    ) -> Result<(), MakeBranchError<L, PackedChildren<L>>> {
        let old_payload_ref = match &self.node().value {
            NodeData::Leaf(val) => val,
            NodeData::Branch {..} => {
                return Err(
                    MakeBranchError {packed_children: children.into()}
                )
            }
        };
        let old_payload = unsafe {
            // SAFETY: both pointer validity and overwriting are upheld
            ptr::read(old_payload_ref)
        };
        let payload = f(old_payload);
        let self_key = self.raw_key().clone();
        let children = children.array_map(
            |value| self.tree.storage.add(
                unsafe {
                    // SAFETY: key validity of self is implied
                    Node::leaf(value, Some(self_key.clone()))
                }
            )
        );
        unsafe {
            // SAFETY: as above
            ptr::write(
                &mut self.node_mut().value,
                NodeData::Branch {children, payload}
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
    #[inline]
    pub fn try_remove_children_with(
        &mut self,
        f: impl FnOnce(B) -> L,
    ) -> Result<[L; 4], TryRemoveChildrenError> {
        let children_keys = {
            let children_keys = match &self.node().value {
                NodeData::Branch { children, .. } => Some(children),
                NodeData::Leaf(..) => None,
            }.ok_or(TryRemoveChildrenError::WasLeafNode)?;
            for (c, i) in children_keys.iter().zip(0_u32..) {
                let child_ref = unsafe {
                    // SAFETY: key validity is assumed, since invalid ones cannot possibly be stored
                    self.tree.storage.get_unchecked(c)
                };
                match &child_ref.value {
                    NodeData::Branch {..} => return Err(TryRemoveChildrenError::HadBranchChild(i)),
                    NodeData::Leaf(..) => {},
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
        let old_payload_ref = match &self.node().value {
            NodeData::Branch { payload, .. } => payload,
            NodeData::Leaf(..) => unsafe {
                // SAFETY: we checked for a leaf node in the beginning
                hint::unreachable_unchecked()
            },
        };
        let old_payload = unsafe {
            // SAFETY: we're overwriting the value later, and not using an invalid pointer
            ptr::read(old_payload_ref)
        };
        unsafe {
            // SAFETY: as above
            ptr::write(
                &mut self.node_mut().value,
                NodeData::Leaf( f(old_payload) ),
            );
        }
        Ok(children_payloads)
    }

    /// Recursively removes the specified node and all its descendants, using a closure to patch nodes which transition from four to zero children.
    #[inline(always)]
    pub fn recursively_remove_with(self, f: impl FnMut(B) -> L) -> NodeValue<B, L> {
        algorithms::recursively_remove_with(self.tree, self.key, f)
    }

    #[inline(always)]
    fn node(&self) -> &'_ Node<B, L, K> {
        unsafe {
            // SAFETY: all existing NodeRefMuts are guaranteed to not be dangling
            self.tree.storage.get_unchecked(&self.key)
        }
    }
    #[inline(always)]
    fn node_mut(&mut self) -> &'_ mut Node<B, L, K> {
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
    #[inline(always)]
    pub fn make_branch(
        &mut self,
        children: [D; 4],
    ) -> Result<(), MakeBranchError<D, PackedChildren<D>>> {
        self.make_branch_with(children, convert::identity)
    }
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, keeping its original payload. Because of that, *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    #[inline(always)]
    pub fn try_remove_children(&mut self) -> Result<[D; 4], TryRemoveChildrenError> {
        self.try_remove_children_with(convert::identity)
    }
    /// Recursively removes the specified node and all its descendants. Will keep the original payload of the parent node if removing this node results in a transformation of the parent into a leaf, which is why *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    #[inline(always)]
    pub fn recursively_remove(self) -> NodeValue<D> {
        algorithms::recursively_remove(self.tree, self.key)
    }
}

impl<'a, B, L, K, S> From<&'a NodeRefMut<'a, B, L, K, S>> for NodeValue<&'a B, &'a L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    #[inline(always)]
    fn from(op: &'a NodeRefMut<'a, B, L, K, S>) -> Self {
        op.value()
    }
}
impl<'a, B, L, K, S> From<&'a mut NodeRefMut<'a, B, L, K, S>> for NodeValue<&'a B, &'a L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    #[inline(always)]
    fn from(op: &'a mut NodeRefMut<'a, B, L, K, S>) -> Self {
        op.value()
    }
}

impl<'a, B, L, K, S> From<&'a mut NodeRefMut<'a, B, L, K, S>> for NodeValue<&'a mut B, &'a mut L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    #[inline(always)]
    fn from(op: &'a mut NodeRefMut<'a, B, L, K, S>) -> Self {
        op.value_mut()
    }
}

impl<'a, B, L, K, S> From<&'a NodeRefMut<'a, B, L, K, S>> for NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    #[inline(always)]
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
    #[inline(always)]
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
    #[inline(always)]
    fn from(op: NodeRefMut<'a, B, L, K, S>) -> Self {
        NodeRef {
            tree: op.tree as &'a _,
            key: op.key,
        }
    }
}