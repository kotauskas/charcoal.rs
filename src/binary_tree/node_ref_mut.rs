use core::{
    ptr,        // write and read
    mem,        // swap
    fmt::Debug, // trait bounds
    hint,       // unreachable_unchecked
    convert,    // identity
};
use crate::{
    storage::{Storage, DefaultStorage},
    util::{unreachable_debugchecked, abort_on_panic},
    TryRemoveLeafError,
    TryRemoveBranchError,
    TryRemoveChildrenError,
    MakeBranchError,
    traversal::algorithms,
    NodeValue,
};
use arrayvec::ArrayVec;
use super::{BinaryTree, MakeFullBranchError, Node, NodeData, NodeRef};

/// A *mutable* reference to a node in a binary tree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree and modify it as a whole.
#[derive(Debug)]
pub struct NodeRefMut<'a, B, L, K, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    tree: &'a mut BinaryTree<B, L, K, S>,
    key: K,
}
impl<'a, B, L, K, S> NodeRefMut<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a new `NodeRefMut` pointing to the specified key in the storage, or `None` if it does not exist.
    pub fn new_raw(tree: &'a mut BinaryTree<B, L, K, S>, key: K) -> Option<Self> {
        if tree.storage.contains_key(&key) {
            Some(unsafe {
                // SAFETY: we just did key checking
                Self::new_raw_unchecked(tree, key)
            })
        } else {
            None
        }
    }
    /// Creates a new `NodeRefMut` pointing to the specified key in the storage without doing key checking.
    ///
    /// # Safety
    /// Causes *immediate* undefined behavior if the specified key is not present in the storage.
    pub unsafe fn new_raw_unchecked(tree: &'a mut BinaryTree<B, L, K, S>, key: K) -> Self {
        Self { tree, key }
    }
    /// Returns a reference to the raw storage key for the node.
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
            // SAFETY: nodes can never have nonexistent parents
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
    #[allow(clippy::missing_const_for_fn)] // const_option is not stable
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
        NodeRef::from(self).is_full_branch()
    }
    /// Returns a reference to the data stored in the node.
    pub fn value(&self) -> NodeValue<&'_ B, &'_ L> {
        self.node().value.as_ref().into_value()
    }
    /// Returns a *mutable* reference to the data stored in the node.
    pub fn value_mut(&mut self) -> NodeValue<&'_ mut B, &'_ mut L> {
        self.node_mut().value.as_mut().into_value()
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
    /// Returns a reference to the left child, or `None` if the node is a leaf node.
    pub fn left_child(&self) -> Option<NodeRef<'_, B, L, K, S>> {
        NodeRef::from(self).left_child()
    }
    /// Returns a *mutable* reference to the left child, or `None` if the node is a leaf node.
    pub fn left_child_mut(&mut self) -> Option<NodeRefMut<'_, B, L, K, S>> {
        if let NodeData::Branch { left_child, .. } = &self.node().value {
            Some(left_child.clone())
        } else {
            None
        }
        .map(move |x| unsafe {
            // SAFETY: child indicies are guaranteed to be valid; a key check to make sure that
            // properly holds is below.
            debug_assert!(
                self.tree.storage.contains_key(&x),
                "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
                &x,
            );
            Self::new_raw_unchecked(self.tree, x)
        })
    }
    /// Returns a reference to the right child, or `None` if the node is a leaf node.
    pub fn right_child(&self) -> Option<NodeRef<'_, B, L, K, S>> {
        NodeRef::from(self).right_child()
    }
    /// Returns a *mutable* reference to the right child, or `None` if the node is a leaf node.
    pub fn right_child_mut(&mut self) -> Option<NodeRefMut<'_, B, L, K, S>> {
        if let NodeData::Branch { left_child, .. } = &self.node().value {
            Some(left_child.clone())
        } else {
            None
        }
        .map(move |x| unsafe {
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

    /// Converts a leaf node into a branch node with the specified leaf children, using the provided closure to convert the payload.
    ///
    /// # Errors
    /// Will fail if the node is already a branch node. In such a case, the provided values for the children are returned back to the caller.
    pub fn make_branch_with(
        &mut self,
        left_child: L,
        right_child: Option<L>,
        f: impl FnOnce(L) -> B,
    ) -> Result<(), MakeBranchError<L, ArrayVec<[L; 2]>>> {
        let old_val_ref = if let NodeData::Leaf(val) = &self.node().value {
            val
        } else {
            return Err(MakeBranchError {
                packed_children: {
                    let mut pack = ArrayVec::new();
                    pack.push(left_child);
                    if let Some(x) = right_child {
                        pack.push(x);
                    }
                    pack
                },
            });
        };
        let old_val = unsafe {
            // SAFETY: the pointer is a valid reference, and we're overwriting the value up next
            ptr::read(old_val_ref)
        };
        let new_val = f(old_val);
        let new_left_child_key = self.tree.storage.add(unsafe {
            // SAFETY: key validity is assumed
            Node::leaf(left_child, Some(self.raw_key().clone()))
        });
        let new_right_child_key = right_child.map(|x| {
            self.tree
                .storage
                .add(unsafe { Node::leaf(x, Some(self.raw_key().clone())) })
        });
        unsafe {
            // SAFETY: see ptr::read safety notes above
            ptr::write(
                &mut self.node_mut().value,
                NodeData::Branch {
                    payload: new_val,
                    left_child: new_left_child_key,
                    right_child: new_right_child_key,
                },
            )
        }
        Ok(())
    }
    /// Converts a partial branch node into a full branch, giving the specified value to the right child.
    ///
    /// # Errors
    /// Will fail if:
    /// - The node was a leaf node — you can use [`make_branch`]/[`make_branch_with`] instead;
    /// - The node already was a full branch.
    ///
    /// In both cases, the provided right child value will not be dropped but instead will be returned to the caller in the error type.
    ///
    /// [`make_branch`]: #method.make_branch " "
    /// [`make_branch_with`]: #method.make_branch_with " "
    pub fn make_full_branch(&mut self, right_child: L) -> Result<(), MakeFullBranchError<L>> {
        match &self.node().value {
            NodeData::Branch {
                right_child: Some(_),
                ..
            } => {
                return Err(MakeFullBranchError::WasFullBranch { right_child });
            }
            NodeData::Branch { .. } => {}
            NodeData::Leaf(_) => {
                return Err(MakeFullBranchError::WasLeafNode { right_child });
            }
        }
        let new_right_child_key = self.tree.storage.add(unsafe {
            // SAFETY: parent validity is assumed via key validity of self
            Node::leaf(right_child, Some(self.raw_key().clone()))
        });
        match &mut self.node_mut().value {
            NodeData::Branch { right_child, .. } => {
                *right_child = Some(new_right_child_key);
            }
            _ => unsafe {
                // SAFETY: leaf check was performed in the beginning
                hint::unreachable_unchecked()
            },
        }
        Ok(())
    }

    /// Attempts to remove a leaf node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure (the previous value is passed into the closure).
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a branch node, which would require recursion to remove, and this function explicitly does not implement recursive removal.
    /// - The node was the root node, which can never be removed.
    pub fn try_remove_leaf_with(self, f: impl FnOnce(B) -> L) -> Result<L, TryRemoveLeafError> {
        if self.is_branch() {
            return Err(TryRemoveLeafError::WasBranchNode);
        }
        let parent_key = self
            .node()
            .parent
            .as_ref()
            .cloned()
            .ok_or(TryRemoveLeafError::WasRootNode)?;
        let (parent_left_child, parent_right_child, parent_payload) = match unsafe {
            // SAFETY: parent key is guaranteed to be valid
            &mut self.tree.storage.get_unchecked_mut(&parent_key).value
        } {
            NodeData::Branch {
                left_child,
                right_child,
                payload,
            } => (left_child, right_child, payload),
            NodeData::Leaf(..) => unsafe {
                unreachable_debugchecked("parent nodes cannot be leaves")
            },
        };
        if &self.key == parent_left_child {
            if let Some(right_child_ref) = parent_right_child {
                mem::swap(parent_left_child, right_child_ref);
                *parent_right_child = None;
            } else {
                let old_payload = unsafe {
                    // SAFETY: the pointer is coerced from a reference and therefore is required to
                    // be valid; we're also overwriting this, so no duplication
                    ptr::read(parent_payload)
                };
                // Destroy the mutable references to modify parent
                drop((parent_left_child, parent_right_child));
                unsafe {
                    // SAFETY: as above
                    ptr::write(
                        &mut self.tree.storage.get_unchecked_mut(&parent_key).value,
                        NodeData::Leaf(abort_on_panic(|| f(old_payload))),
                    );
                }
            }
        } else if Some(&self.key) == parent_right_child.as_ref() {
            *parent_right_child = None;
        } else {
            unsafe {
                // SAFETY: a node cannot have a parent which does not list it as one
                // of its children
                unreachable_debugchecked(
                    "failed to identify whether the node is the left or right child",
                )
            }
        }
        let key = self.key.clone();
        match self.tree.storage.remove(&key).value {
            NodeData::Leaf(x) => Ok(x),
            NodeData::Branch { .. } => unsafe {
                // SAFETY: the beggining of the function tests for self being a branch node
                hint::unreachable_unchecked()
            },
        }
    }
    /// Attempts to remove a branch node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure (the previous value is passed into the closure).
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node. The `try_remove_leaf`/`try_remove_leaf_with` methods exist for that.
    /// - The node was the root node, which can never be removed.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_branch_with(
        self,
        f: impl FnOnce(B) -> L,
    ) -> Result<(B, L, Option<L>), TryRemoveBranchError> {
        if let NodeData::Branch {
            left_child,
            right_child,
            ..
        } = &self.node().value
        {
            let (left_child_ref, right_child_ref) = unsafe {
                // SAFETY: both keys are required to be valid
                (
                    NodeRef::new_raw_unchecked(self.tree, left_child.clone()),
                    right_child.as_ref().map(|right_child| {
                        NodeRef::new_raw_unchecked(self.tree, right_child.clone())
                    }),
                )
            };
            if left_child_ref.is_branch() {
                return Err(TryRemoveBranchError::HadBranchChild(0));
            } else if right_child_ref.as_ref().map(NodeRef::is_branch) == Some(true) {
                return Err(TryRemoveBranchError::HadBranchChild(1));
            }
        } else {
            return Err(TryRemoveBranchError::WasLeafNode);
        }
        let parent_key = self
            .node()
            .parent
            .as_ref()
            .cloned()
            .ok_or(TryRemoveBranchError::WasRootNode)?;
        let (parent_left_child, parent_right_child, parent_payload) = match unsafe {
            // SAFETY: parent key is guaranteed to be valid
            &mut self.tree.storage.get_unchecked_mut(&parent_key).value
        } {
            NodeData::Branch {
                left_child,
                right_child,
                payload,
            } => (left_child, right_child, payload),
            NodeData::Leaf(..) => unsafe {
                unreachable_debugchecked("parent nodes cannot be leaves")
            },
        };
        if &self.key == parent_left_child {
            if let Some(parent_right_child_ref) = parent_right_child {
                mem::swap(parent_left_child, parent_right_child_ref);
                *parent_right_child = None;
            } else {
                let old_payload = unsafe {
                    // SAFETY: the pointer is coerced from a reference and therefore is required to
                    // be valid; we're also overwriting this, so no duplication
                    ptr::read(parent_payload)
                };
                // Destroy the mutable references to modify parent
                drop((parent_left_child, parent_right_child));
                unsafe {
                    // SAFETY: as above
                    ptr::write(
                        &mut self.tree.storage.get_unchecked_mut(&parent_key).value,
                        NodeData::Leaf(abort_on_panic(|| f(old_payload))),
                    );
                }
            }
        } else if Some(&self.key) == parent_right_child.as_ref() {
            *parent_right_child = None;
        } else {
            unsafe {
                // SAFETY: a node cannot have a parent which does not list it as one
                // of its children
                unreachable_debugchecked(
                    "failed to identify whether the node is the left or right child",
                )
            }
        }
        let key = self.key.clone();
        let (payload, left_child_key, right_child_key) = match self.tree.storage.remove(&key).value
        {
            NodeData::Branch {
                payload,
                left_child: left_child_key,
                right_child: right_child_key,
            } => (payload, left_child_key, right_child_key),
            NodeData::Leaf(..) => unsafe {
                // SAFETY: the beggining of the function tests for self being a branch node
                hint::unreachable_unchecked()
            },
        };
        let left_child_payload = match self.tree.storage.remove(&left_child_key).value {
            NodeData::Leaf(x) => x,
            NodeData::Branch { .. } => unsafe {
                // SAFETY: a check for branch children was made at the beginning
                hint::unreachable_unchecked()
            },
        };
        let right_child_payload = right_child_key.map(|right_child_key| {
            match self.tree.storage.remove(&right_child_key).value {
                NodeData::Leaf(x) => x,
                NodeData::Branch { .. } => unsafe {
                    // SAFETY: as above
                    hint::unreachable_unchecked()
                },
            }
        });
        Ok((payload, left_child_payload, right_child_payload))
    }
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, the value for which is provided by the specified closure.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_children_with(
        &mut self,
        f: impl FnOnce(B) -> L,
    ) -> Result<(L, Option<L>), TryRemoveChildrenError> {
        let (left_child_key, right_child_key, ..) = if let NodeData::Branch {
            left_child,
            right_child,
            ..
        } = &self.node().value
        {
            let (left_child_ref, right_child_ref) = unsafe {
                // SAFETY: both keys are required to be valid
                (
                    NodeRef::new_raw_unchecked(self.tree, left_child.clone()),
                    right_child.as_ref().map(|right_child| {
                        NodeRef::new_raw_unchecked(self.tree, right_child.clone())
                    }),
                )
            };
            if left_child_ref.is_branch() {
                return Err(TryRemoveChildrenError::HadBranchChild(0));
            } else if right_child_ref.as_ref().map(NodeRef::is_branch) == Some(true) {
                return Err(TryRemoveChildrenError::HadBranchChild(1));
            }
            (left_child_ref.key, right_child_ref.map(|x| x.key))
        } else {
            return Err(TryRemoveChildrenError::WasLeafNode);
        };
        let left_child_payload = match self.tree.storage.remove(&left_child_key).value {
            NodeData::Leaf(x) => x,
            NodeData::Branch { .. } => unsafe {
                // SAFETY: a check for branch children was made at the beginning
                hint::unreachable_unchecked()
            },
        };
        let right_child_payload = right_child_key.map(|right_child_key| {
            match self.tree.storage.remove(&right_child_key).value {
                NodeData::Leaf(x) => x,
                NodeData::Branch { .. } => unsafe {
                    // SAFETY: as above
                    hint::unreachable_unchecked()
                },
            }
        });
        let old_payload_ref = match &mut self.node_mut().value {
            NodeData::Branch { payload, .. } => payload,
            NodeData::Leaf(..) => unsafe {
                // SAFETY: we checked for a leaf node in the beginning
                hint::unreachable_unchecked()
            },
        };
        let old_payload = unsafe {
            // SAFETY: the pointer is coerced from a reference and therefore is required to
            // be valid; we're also overwriting this, so no duplication
            ptr::read(old_payload_ref)
        };
        unsafe {
            // SAFETY: as above
            ptr::write(
                &mut self.node_mut().value,
                NodeData::Leaf(abort_on_panic(|| f(old_payload))),
            );
        }
        Ok((left_child_payload, right_child_payload))
    }
    /// Recursively removes the specified node and all its descendants, using a closure to patch nodes which transition from having one child to having zero children.
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
        left_child: D,
        right_child: Option<D>,
    ) -> Result<(), MakeBranchError<D, ArrayVec<[D; 2]>>> {
        self.make_branch_with(left_child, right_child, convert::identity)
    }

    /// Attempts to remove the node without using recursion. If the parent only had one child, it's replaced with a leaf node, keeping its original payload, which is why *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a branch node, which would require recursion to remove, and this function explicitly does not implement recursive removal.
    /// - The node was the root node, which can never be removed.
    pub fn try_remove_leaf(self) -> Result<D, TryRemoveLeafError> {
        self.try_remove_leaf_with(convert::identity)
    }
    /// Attempts to remove a branch node without using recursion. If its parent only had one child, it's replaced with a leaf node, keeping its original payload, which is why *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node. The `try_remove_leaf`/`try_remove_leaf_with` methods exist for that.
    /// - The node was the root node, which can never be removed.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_branch(self) -> Result<(D, D, Option<D>), TryRemoveBranchError> {
        self.try_remove_branch_with(convert::identity)
    }
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, keeping its original payload. Because of that, *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_children(&mut self) -> Result<(D, Option<D>), TryRemoveChildrenError> {
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
