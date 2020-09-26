use core::{
    ptr, // write and read
    mem, // swap
    fmt::Debug, // trait bounds
    hint, // unreachable_unchecked
    convert, // identity
};
use crate::{
    TryRemoveLeafError, TryRemoveBranchError, TryRemoveChildrenError,
    storage::{Storage, DefaultStorage},
    traversal::algorithms,
    NodeValue,
};
use arrayvec::ArrayVec;
use super::{BinaryTree, Node, NodeData};

/// A reference to a node in a binary tree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree.
#[derive(Debug)]
pub struct NodeRef<'a, B, L, K, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    tree: &'a BinaryTree<B, L, K, S>,
    key: K,
}
impl<'a, B, L, K, S> NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    /// Creates a new `NodeRef` pointing to the specified key in the storage, or `None` if it's out of bounds.
    #[inline]
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
    #[inline(always)]
    pub unsafe fn new_raw_unchecked(tree: &'a BinaryTree<B, L, K, S>, key: K) -> Self {
        Self {tree, key}
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
    #[allow(clippy::missing_const_for_fn)] // const_option is not stable
    pub fn is_root(&self) -> bool {
        self.node().parent.is_none()
    }
    /// Returns `true` if the node is a *leaf*, i.e. does not have child nodes; `false` otherwise.
    #[inline(always)]
    pub fn is_leaf(&self) -> bool {
        !self.is_branch()
    }
    /// Returns `true` if the node is a *branch*, i.e. has one or two child nodes; `false` otherwise.
    #[inline(always)]
    pub fn is_branch(&self) -> bool {
        self.left_child().is_some()
    }
    /// Returns `true` if the node is a *full branch*, i.e. has exactly two child nodes; `false` otherwise.
    #[inline(always)]
    pub fn is_full_branch(&self) -> bool {
        self.children().is_some()
    }
    /// Returns a reference to the data stored in the node.
    #[inline(always)]
    pub fn value(&self) -> NodeValue<&'a B, &'a L> {
        self.node().value.as_ref().into_value()
    }
    /// Returns references to the children, or `None` if the node is a leaf node or it only has one child. To retreive the left child even if the right one is not present, see `left_child`.
    pub fn children(&self) -> Option<(Self, Self)> {
        match &self.node().value {
            NodeData::Branch {left_child, right_child, ..} => {
                right_child.as_ref().map(|right_child| (left_child.clone(), right_child.clone()))
            },
            NodeData::Leaf(..) => None,
        }.map(|(left_child, right_child)| unsafe {
            // SAFETY: child keys are guaranteed to be valid; a key check to make sure that
            // properly holds is below.
            debug_assert!(
                   self.tree.storage.contains_key(&left_child)
                && self.tree.storage.contains_key(&right_child), "\
debug key check failed: tried to reference keys {:?} and {:?} which are not present in the storage",
                &left_child, &right_child,
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
    pub fn left_child(&self) -> Option<Self> {
        match &self.node().value {
            NodeData::Branch {left_child, ..} => Some(left_child),
            NodeData::Leaf(..) => None,
        }.map(|x| unsafe {
            // SAFETY: child keys are guaranteed to be valid; a key check to make sure that
            // properly holds is below.
            debug_assert!(
                self.tree.storage.contains_key(x), "\
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
    pub fn right_child(&self) -> Option<Self> {
        match &self.node().value {
            NodeData::Branch {right_child, ..} => right_child.clone(),
            NodeData::Leaf(..) => None,
        }.map(|x| unsafe {
            // SAFETY: as above
            debug_assert!(
                self.tree.storage.contains_key(&x), "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
                &x,
            );
            Self::new_raw_unchecked(self.tree, x)
        })
    }

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
    K: Copy + Debug + Eq {}
impl<B, L, K, S> Clone for NodeRef<'_, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
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
    K: Clone + Debug + Eq {
    #[inline(always)]
    fn from(op: NodeRef<'a, B, L, K, S>) -> Self {
        op.value()
    }
}

/// A *mutable* reference to a node in a binary tree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree and modify it as a whole.
#[derive(Debug)]
pub struct NodeRefMut<'a, B, L, K, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    tree: &'a mut BinaryTree<B, L, K, S>,
    key: K,
}
impl<'a, B, L, K, S> NodeRefMut<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    /// Creates a new `NodeRefMut` pointing to the specified key in the storage, or `None` if it does not exist.
    #[inline(always)]
    pub fn new_raw(tree: &'a mut BinaryTree<B, L, K, S>, key: K) -> Option<Self> {
        if tree.storage.contains_key(&key) {
            Some(unsafe {
                // SAFETY: we just did key checking
                Self::new_raw_unchecked(tree, key)
            })
        } else {None}
    }
    /// Creates a new `NodeRefMut` pointing to the specified key in the storage without doing key checking.
    ///
    /// # Safety
    /// Causes *immediate* undefined behavior if the specified key is not present in the storage.
    #[inline(always)]
    pub unsafe fn new_raw_unchecked(tree: &'a mut BinaryTree<B, L, K, S>, key: K) -> Self {
        Self {tree, key}
    }
    /// Returns a reference to the raw storage key for the node.
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
    pub fn parent(&'a self) -> Option<NodeRef<'a, B, L, K, S>> {
        self.node().parent.as_ref().map(|x| unsafe {
            // SAFETY: nodes can never have nonexistent parents
            NodeRef::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns a *mutable* reference to the parent node of the pointee, or `None` if it's the root node.
    #[inline]
    pub fn parent_mut(&'a mut self) -> Option<Self> {
        let key = self.node().parent.as_ref().cloned();
        key.map(move |x| unsafe {
            // SAFETY: as above
            Self::new_raw_unchecked(self.tree, x)
        })
    }
    /// Returns `true` if the node is the root node, `false` otherwise.
    #[inline(always)]
    #[allow(clippy::missing_const_for_fn)] // const_option is not stable
    pub fn is_root(&self) -> bool {
        self.node().parent.is_none()
    }
    /// Returns `true` if the node is a *leaf*, i.e. does not have child nodes; `false` otherwise.
    #[inline(always)]
    pub fn is_leaf(&self) -> bool {
        !self.is_branch()
    }
    /// Returns `true` if the node is a *branch*, i.e. has one or two child nodes; `false` otherwise.
    #[inline(always)]
    pub fn is_branch(&self) -> bool {
        self.left_child().is_some()
    }
    /// Returns `true` if the node is a *full branch*, i.e. has exactly two child nodes; `false` otherwise.
    #[inline(always)]
    pub fn is_full_branch(&self) -> bool {
        NodeRef::from(self).is_full_branch()
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
    /// Returns a reference to the left child, or `None` if the node is a leaf node.
    pub fn left_child(&'a self) -> Option<NodeRef<'a, B, L, K, S>> {
        NodeRef::from(self).left_child()
    }
    /// Returns a *mutable* reference to the left child, or `None` if the node is a leaf node.
    pub fn left_child_mut(&'a mut self) -> Option<Self> {
        match &self.node().value {
            NodeData::Branch {left_child, ..} => Some(left_child.clone()),
            NodeData::Leaf(..) => None,
        }.map(move |x| unsafe {
            // SAFETY: child indicies are guaranteed to be valid; a key check to make sure that
            // properly holds is below.
            debug_assert!(
                self.tree.storage.contains_key(&x), "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
                &x,
            );
            Self::new_raw_unchecked(self.tree, x)
        })
    }
    /// Returns a reference to the right child, or `None` if the node is a leaf node.
    pub fn right_child(&'a self) -> Option<NodeRef<'a, B, L, K, S>> {
        NodeRef::from(self).right_child()
    }
    /// Returns a *mutable* reference to the right child, or `None` if the node is a leaf node.
    pub fn right_child_mut(&'a mut self) -> Option<Self> {
        match &self.node().value {
            NodeData::Branch {right_child, ..} => right_child.clone(),
            NodeData::Leaf(..) => None,
        }.map(move |x| unsafe {
            // SAFETY: as above
            debug_assert!(
                self.tree.storage.contains_key(&x), "\
debug key check failed: tried to reference key {:?} which is not present in the storage",
                &x,
            );
            Self::new_raw_unchecked(self.tree, x)
        })
    }
    /// Attempts to remove a leaf node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure (the previous value is passed into the closure).
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a branch node, which would require recursion to remove, and this function explicitly does not implement recursive removal.
    /// - The node was the root node, which can never be removed.
    pub fn try_remove_leaf_with<F>(&mut self, f: F) -> Result<L, TryRemoveLeafError>
    where F: FnOnce(B) -> L {
        if matches!(&self.node().value, NodeData::Branch {..}) {
            return Err(TryRemoveLeafError::WasBranchNode)
        }
        let parent_key = self.node()
            .parent
            .as_ref()
            .cloned()
            .ok_or(TryRemoveLeafError::WasRootNode)?;
        let (parent_left_child, parent_right_child, parent_payload) = match unsafe {
            // SAFETY: parent key is guaranteed to be valid
            &mut self.tree.storage.get_unchecked_mut(&parent_key).value
        } {
            NodeData::Branch {left_child, right_child, payload} => (
                left_child,
                right_child,
                payload,
            ),
            NodeData::Leaf(..) => unsafe {
                // SAFETY: cannot have leaf node as parent
                hint::unreachable_unchecked()
            }
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
                        NodeData::Leaf(f(old_payload)),
                    );
                }
            }
        } else if Some(&self.key) == parent_right_child.as_ref() {
            *parent_right_child = None;
        } else {
            unsafe {
                // SAFETY: a node cannot have a parent which does not list it as one
                // of its children
                if cfg!(debug_assertions) {
                    panic!("failed to identify whether the node is the left or right child");
                }
                hint::unreachable_unchecked()
            }
        }
        let key = self.key.clone();
        match self.tree.storage.remove(&key).value {
            NodeData::Leaf(x) => Ok(x),
            NodeData::Branch {..} => unsafe {
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
    pub fn try_remove_branch_with<F>(
        &mut self,
        f: F,
    ) -> Result<(B, L, Option<L>), TryRemoveBranchError>
    where F: FnOnce(B) -> L {
        match &self.node().value {
            NodeData::Branch {left_child, right_child, ..} => {
                let (left_child_ref, right_child_ref) = unsafe {
                    // SAFETY: both keys are required to be valid
                    (
                        NodeRef::new_raw_unchecked(self.tree, left_child.clone()),
                        right_child.as_ref().map(
                            |right_child| NodeRef::new_raw_unchecked(
                                self.tree,
                                right_child.clone(),
                            )
                        ),
                    )
                };
                if left_child_ref.is_branch() {
                    return Err(TryRemoveBranchError::HadBranchChild(0))
                } else if right_child_ref.as_ref().map(NodeRef::is_branch) == Some(true) {
                    return Err(TryRemoveBranchError::HadBranchChild(1))
                }
            },
            NodeData::Leaf(..) => return Err(TryRemoveBranchError::WasLeafNode),
        }
        let parent_key = self.node()
            .parent
            .as_ref()
            .cloned()
            .ok_or(TryRemoveBranchError::WasRootNode)?;
        let (parent_left_child, parent_right_child, parent_payload) = match unsafe {
            // SAFETY: parent key is guaranteed to be valid
            &mut self.tree.storage.get_unchecked_mut(&parent_key).value
        } {
            NodeData::Branch {left_child, right_child, payload} => (
                left_child,
                right_child,
                payload,
            ),
            NodeData::Leaf(..) => unsafe {
                // SAFETY: cannot have leaf node as parent
                hint::unreachable_unchecked()
            }
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
                        NodeData::Leaf(f(old_payload)),
                    );
                }
            }
        } else if Some(&self.key) == parent_right_child.as_ref() {
            *parent_right_child = None;
        } else {
            unsafe {
                // SAFETY: a node cannot have a parent which does not list it as one
                // of its children
                if cfg!(debug_assertions) {
                    panic!("failed to identify whether the node is the left or right child");
                }
                hint::unreachable_unchecked()
            }
        }
        let key = self.key.clone();
        let (
            payload,
            left_child_key,
            right_child_key,
        ) = match self.tree.storage.remove(&key).value {
            NodeData::Branch {
                payload,
                left_child: left_child_key,
                right_child: right_child_key,
            } => (
                payload,
                left_child_key,
                right_child_key,
            ),
            NodeData::Leaf(..) => unsafe {
                // SAFETY: the beggining of the function tests for self being a branch node
                hint::unreachable_unchecked()
            },
        };
        let left_child_payload = match self.tree.storage.remove(&left_child_key).value {
            NodeData::Leaf(x) => x,
            NodeData::Branch {..} => unsafe {
                // SAFETY: a check for branch children was made at the beginning
                hint::unreachable_unchecked()
            },
        };
        let right_child_payload = right_child_key.map(|right_child_key| {
            match self.tree.storage.remove(&right_child_key).value {
                NodeData::Leaf(x) => x,
                NodeData::Branch {..} => unsafe {
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
    pub fn try_remove_children_with<F>(
        &mut self,
        f: F,
    ) -> Result<(L, Option<L>), TryRemoveChildrenError>
    where F: FnOnce(B) -> L {
        let (left_child_key, right_child_key, ..) = match &self.node().value {
            NodeData::Branch {left_child, right_child, ..} => {
                let (left_child_ref, right_child_ref) = unsafe {
                    // SAFETY: both keys are required to be valid
                    (
                        NodeRef::new_raw_unchecked(self.tree, left_child.clone()),
                        right_child.as_ref().map(
                            |right_child| NodeRef::new_raw_unchecked(
                                self.tree,
                                right_child.clone(),
                            )
                        ),
                    )
                };
                if left_child_ref.is_branch() {
                    return Err(TryRemoveChildrenError::HadBranchChild(0))
                } else if right_child_ref.as_ref().map(NodeRef::is_branch) == Some(true) {
                    return Err(TryRemoveChildrenError::HadBranchChild(1))
                }
                (
                    left_child_ref.key,
                    right_child_ref.map(|x| x.key),
                )
            },
            NodeData::Leaf(..) => return Err(TryRemoveChildrenError::WasLeafNode),
        };
        let left_child_payload = match self.tree.storage.remove(&left_child_key).value {
            NodeData::Leaf(x) => x,
            NodeData::Branch {..} => unsafe {
                // SAFETY: a check for branch children was made at the beginning
                hint::unreachable_unchecked()
            },
        };
        let right_child_payload = right_child_key.map(|right_child_key| {
            match self.tree.storage.remove(&right_child_key).value {
                NodeData::Leaf(x) => x,
                NodeData::Branch {..} => unsafe {
                    // SAFETY: as above
                    hint::unreachable_unchecked()
                },
            }
        });
        let old_payload_ref = match &mut self.node_mut().value {
            NodeData::Branch {payload, ..} => payload,
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
            ptr::write(&mut self.node_mut().value, NodeData::Leaf(f(old_payload)));
        }
        Ok((left_child_payload, right_child_payload))
    }
    /// Recursively removes the specified node and all its descendants, using a closure to patch nodes which transition from having one child to having zero children.
    #[inline(always)]
    pub fn recursively_remove_with<F: FnMut(B) -> L>(self, f: F) -> NodeValue<B, L> {
        algorithms::recursively_remove_with(self.tree, self.key, f)
    }
    /// Sets the children of the node to specified zero, one or two children. If there were children before and the method would otherwise drop them, they are instead bundled and returned.
    ///
    /// If the node was a leaf before and adding one or more children is requested, the old value is passed to first closure and replaced with a new branch payload; if it was a branch before and removing all children is requested, the old value is passed to the second closure and replaced with a new leaf payload.
    #[allow(clippy::shadow_unrelated)] // bullshit lint
    pub fn set_children_with<LtB: FnMut(L) -> B, BtL: FnMut(B) -> L>(
        &'_ mut self,
        new_children: ArrayVec<[L; 2]>,
        mut leaf_to_branch: LtB,
        mut branch_to_leaf: BtL,
    ) -> ArrayVec<[NodeValue<B, L>; 2]> {
        let (new_left_child, new_right_child) = {
            let mut new_children = new_children.into_iter();
            (
                new_children.next(),
                new_children.next(),
            )
        };
        let new_left_child_key = new_left_child.map(|new_left_child| {
            self.tree.storage.add(
                unsafe {
                    // SAFETY: the following invariants are upheld:
                    // - we're evidently not creating a second root node
                    // - parent is valid because the key of self is assumed to be valid
                    Node::leaf(new_left_child, Some(self.key.clone()))
                }
            )
        });
        let new_right_child_key = new_right_child.map(|new_right_child| {
            self.tree.storage.add(
                unsafe {
                    // SAFETY: as above
                    Node::leaf(new_right_child, Some(self.key.clone()))
                }
            )
        });
        let new_children_keys = new_left_child_key.map(|new_left_child_key| (
            new_left_child_key,
            new_right_child_key,
        ));
        let node = self.node_mut();
        let parent = node.parent.clone();
        match &mut node.value {
            NodeData::Branch {payload, left_child, right_child} => {
                let old_val_owned = unsafe {
                    // SAFETY: we're overwriting this afterwards
                    ptr::read(payload)
                };
                // why do a whole ptr::read when you can do something that results
                // exactly the same in codegen?
                let left_child = left_child.clone();
                let right_child = right_child.clone();
                let left_child_owned = algorithms::recursively_remove_with(
                    self.tree,
                    left_child,
                    &mut branch_to_leaf,
                );
                let right_child_owned = right_child.map(|right_child| {
                    algorithms::recursively_remove_with(
                        self.tree,
                        right_child,
                        &mut branch_to_leaf,
                    )
                });
                let node = self.node_mut();
                unsafe {
                    // SAFETY: pointer comes from a reference and therefore is valid
                    ptr::write::<Node<B, L, K>>(
                        node,
                        match new_children_keys {
                            Some((l, Some(r))) => {
                                Node::full_branch(old_val_owned, [l, r], parent)
                            },
                            Some((c, None)) => {
                                Node::partial_branch(old_val_owned, c, parent)
                            },
                            None => {
                                Node::leaf(branch_to_leaf(old_val_owned), parent)
                            },
                        },
                    );
                }
                let mut old_children = ArrayVec::new();
                old_children.push(left_child_owned);
                if let Some(right_child_owned) = right_child_owned {
                    old_children.push(right_child_owned);
                }
                old_children
            },
            NodeData::Leaf(old_val) => {
                let old_val_owned = unsafe {
                    // SAFETY: we're overwriting this right below
                    ptr::read(old_val)
                };
                unsafe {
                    // SAFETY: pointer comes from a reference and therefore is valid
                    ptr::write::<Node<B, L, K>>(
                        node,
                        match new_children_keys {
                            Some((l, Some(r))) => {
                                Node::full_branch(leaf_to_branch(old_val_owned), [l, r], parent)
                            },
                            Some((c, None)) => {
                                Node::partial_branch(leaf_to_branch(old_val_owned), c, parent)
                            },
                            None => {
                                Node::leaf(old_val_owned, parent)
                            },
                        },
                    );
                }
                // There were no children, so we're returning an empty bundle
                ArrayVec::new()
            },
        }
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
    K: Clone + Debug + Eq {
    /// Attempts to remove the node without using recursion. If the parent only had one child, it's replaced with a leaf node, keeping its original payload, which is why *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a branch node, which would require recursion to remove, and this function explicitly does not implement recursive removal.
    /// - The node was the root node, which can never be removed.
    #[inline(always)]
    pub fn try_remove_leaf(&mut self) -> Result<D, TryRemoveLeafError> {
        self.try_remove_leaf_with(convert::identity)
    }
    /// Attempts to remove a branch node without using recursion. If its parent only had one child, it's replaced with a leaf node, keeping its original payload, which is why *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node. The `try_remove_leaf`/`try_remove_leaf_with` methods exist for that.
    /// - The node was the root node, which can never be removed.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    #[inline(always)]
    pub fn try_remove_branch(
        &mut self,
    ) -> Result<(D, D, Option<D>), TryRemoveBranchError> {
        self.try_remove_branch_with(convert::identity)
    }
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, keeping its original payload. Because of that, *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    #[inline(always)]
    pub fn try_remove_children(
        &mut self,
    ) -> Result<(D, Option<D>), TryRemoveChildrenError> {
        self.try_remove_children_with(convert::identity)
    }
    /// Recursively removes the specified node and all its descendants. Will keep the original payload of the parent node if removing this node results in a transformation of the parent into a leaf, which is why *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    #[inline(always)]
    pub fn recursively_remove(self) -> NodeValue<D> {
        algorithms::recursively_remove(self.tree, self.key)
    }
    /// Sets the children of the node to specified zero, one or two children. If there were children before and the method would otherwise drop them, they are instead bundled and returned.
    ///
    /// If any nodes transition from leaf to branch or the other way around, their payload is preserved, which is why *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    #[inline(always)]
    pub fn set_children(
        &'_ mut self,
        new_children: ArrayVec<[D; 2]>,
    ) -> ArrayVec<[NodeValue<D>; 2]> {
        self.set_children_with(new_children, convert::identity, convert::identity)
    }
}
impl<'a, B, L, K, S> From<&'a NodeRefMut<'a, B, L, K, S>> for NodeValue<&'a B, &'a L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    #[inline(always)]
    fn from(op: &'a NodeRefMut<'a, B, L, K, S>) -> Self {
        op.value()
    }
}
impl<'a, B, L, K, S> From<&'a mut NodeRefMut<'a, B, L, K, S>> for NodeValue<&'a B, &'a L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    #[inline(always)]
    fn from(op: &'a mut NodeRefMut<'a, B, L, K, S>) -> Self {
        op.value()
    }
}

impl<'a, B, L, K, S> From<&'a mut NodeRefMut<'a, B, L, K, S>> for NodeValue<&'a mut B, &'a mut L>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
    #[inline(always)]
    fn from(op: &'a mut NodeRefMut<'a, B, L, K, S>) -> Self {
        op.value_mut()
    }
}

impl<'a, B, L, K, S> From<&'a NodeRefMut<'a, B, L, K, S>> for NodeRef<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq {
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
    K: Clone + Debug + Eq {
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
    K: Clone + Debug + Eq {
    #[inline(always)]
    fn from(op: NodeRefMut<'a, B, L, K, S>) -> Self {
        NodeRef {
            tree: op.tree as &'a _,
            key: op.key,
        }
    }
}