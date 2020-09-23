use core::{
    ptr,
    mem,
    fmt::Debug,
    hint,
};
use crate::{
    TryRemoveLeafError, TryRemoveBranchError, TryRemoveChildrenError,
    storage::{Storage, DefaultStorage},
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
    /// Returns the raw storage key for the node.
    #[inline(always)]
    pub fn raw_key(&self) -> K {
        self.key.clone()
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
    /// Returns the raw storage key for the node.
    #[inline(always)]
    pub fn raw_key(&self) -> K {
        self.key.clone()
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
    /// Attempts to remove a leaf node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a branch node, which would require recursion to remove, and this function explicitly does not implement recursive removal.
    /// - The node was the root node, which can never be removed.
    pub fn try_remove_leaf_with<F>(self, f: F) -> Result<L, TryRemoveLeafError>
    where F: FnOnce() -> L {
        if matches!(&self.node().value, NodeData::Branch {..}) {
            return Err(TryRemoveLeafError::WasBranchNode)
        }
        let parent_key = self.node()
            .parent
            .as_ref()
            .cloned()
            .ok_or(TryRemoveLeafError::WasRootNode)?;
        let (left_child, right_child) = match unsafe {
            // SAFETY: parent key is guaranteed to be valid
            &mut self.tree.storage.get_unchecked_mut(&parent_key).value
        } {
            NodeData::Branch {left_child, right_child, ..} => (left_child, right_child),
            NodeData::Leaf(..) => unsafe {
                // SAFETY: cannot have leaf node as parent
                hint::unreachable_unchecked()
            }
        };
        if &self.key == left_child {
            if let Some(right_child_ref) = right_child {
                mem::swap(left_child, right_child_ref);
                *right_child = None;
            } else {
                // Destroy the mutable references to modify parent
                drop((left_child, right_child));
                *unsafe {
                    // SAFETY: as above
                    &mut self.tree.storage.get_unchecked_mut(&parent_key).value
                } = NodeData::Leaf(f());
            }
        } else if Some(&self.key) == right_child.as_ref() {
            *right_child = None;
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
    /// Attempts to remove a branch node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure.
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
    where F: FnOnce() -> L {
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
                if left_child_ref.is_branch()
                        || (right_child_ref.map(|x| x.is_branch()) == Some(true)) {
                    return Err(TryRemoveBranchError::HadBranchChild)
                }
            },
            NodeData::Leaf(..) => return Err(TryRemoveBranchError::WasLeafNode),
        }
        let parent_key = self.node()
            .parent
            .as_ref()
            .cloned()
            .ok_or(TryRemoveBranchError::WasRootNode)?;
        let (left_child, right_child) = match unsafe {
            // SAFETY: parent key is guaranteed to be valid
            &mut self.tree.storage.get_unchecked_mut(&parent_key).value
        } {
            NodeData::Branch {left_child, right_child, ..} => (left_child, right_child),
            NodeData::Leaf(..) => unsafe {
                // SAFETY: cannot have leaf node as parent
                hint::unreachable_unchecked()
            }
        };
        if &self.key == left_child {
            if let Some(right_child_ref) = right_child {
                mem::swap(left_child, right_child_ref);
                *right_child = None;
            } else {
                // Destroy the mutable references to modify parent
                drop((left_child, right_child));
                *unsafe {
                    // SAFETY: as above
                    &mut self.tree.storage.get_unchecked_mut(&parent_key).value
                } = NodeData::Leaf(f());
            }
        } else if Some(&self.key) == right_child.as_ref() {
            *right_child = None;
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
    ) -> Result<(B, L, Option<L>), TryRemoveChildrenError>
    where F: FnOnce() -> L {
        let (left_child_key, right_child_key) = match &self.node().value {
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
                if left_child_ref.is_branch()
                    || (right_child_ref.as_ref().map(NodeRef::is_branch) == Some(true)) {
                    return Err(TryRemoveChildrenError::HadBranchChild)
                }
                (left_child_ref.key, right_child_ref.map(|x| x.key))
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
        let old_value = mem::replace(&mut self.node_mut().value, NodeData::Leaf(f()));
        let old_payload = match old_value {
            NodeData::Branch {payload, ..} => payload,
            NodeData::Leaf(..) => unsafe {
                // SAFETY: we checked for a leaf node in the beginning
                hint::unreachable_unchecked()
            },
        };
        Ok((old_payload, left_child_payload, right_child_payload))
    }
    /// Adds two leaf children to the node, or, if it already has one or two, overwrites the existing ones. If the node was a leaf before, the old value is returned and replaced with a new branch payload; if it was a branch, the old branch value is returned.
    pub fn set_children_and_payload(&'_ mut self, new_payload: B, new_children: [L; 2]) -> NodeValue<B, L> {
        let [new_left_child, new_right_child] = new_children;
        let left_child_key = self.tree.storage.add(
            unsafe {
                // SAFETY: the following invariants are upheld:
                // - we're evidently not creating a second root node
                // - parent is valid because the key of self is assumed to be valid
                Node::leaf(new_left_child, Some(self.key.clone()))
            }
        );
        let right_child_key = self.tree.storage.add(
            unsafe {
                // SAFETY: as above
                Node::leaf(new_right_child, Some(self.key.clone()))
            }
        );
        let node = self.node_mut();
        let parent = node.parent.clone();
        match &mut node.value {
            NodeData::Branch {payload, left_child, right_child} => todo!(), // TODO
            NodeData::Leaf(old_val) => {
                let old_val_owned = unsafe {
                    // SAFETY: we're overwriting this right below
                    ptr::read(old_val)
                };
                unsafe {
                    // SAFETY: pointer comes from a reference and therefore is valid
                    ptr::write::<Node<B, L, K>>(
                        node,
                        Node::full_branch(
                            new_payload,
                            [left_child_key, right_child_key],
                            parent,
                        ),
                    );
                }
                NodeValue::Leaf(old_val_owned)
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
    /// Attempts to remove the node without using recursion. If the parent only had one child, it's replaced with a leaf node, keeping its original payload. *This method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a branch node, which would require recursion to remove, and this function explicitly does not implement recursive removal.
    /// - The node was the root node, which can never be removed.
    pub fn try_remove_leaf(self) -> Result<D, TryRemoveLeafError> {
        if matches!(&self.node().value, NodeData::Branch {..}) {
            return Err(TryRemoveLeafError::WasBranchNode)
        }
        let parent_key = self.node()
            .parent
            .as_ref()
            .cloned()
            .ok_or(TryRemoveLeafError::WasRootNode)?;
        let (left_child, right_child, payload) = match unsafe {
            // SAFETY: parent key is guaranteed to be valid
            &mut self.tree.storage.get_unchecked_mut(&parent_key).value
        } {
            NodeData::Branch {left_child, right_child, payload} => {
                (left_child, right_child, payload)
            }
            NodeData::Leaf(..) => unsafe {
                // SAFETY: cannot have leaf node as parent
                hint::unreachable_unchecked()
            },
        };
        if &self.key == left_child {
            if let Some(right_child_ref) = right_child {
                mem::swap(left_child, right_child_ref);
                *right_child = None;
            } else {
                // Destroy the mutable references to modify parent
                drop((left_child, right_child));
                let payload = unsafe {
                    // SAFETY: we're overwriting the memory right afterwards, and
                    // the pointer comes from a reference and therefore is valid
                    ptr::read(payload)
                };
                unsafe {
                    // SAFETY: we're overwriting what we just read, which is
                    // still behind a reference and still valid
                    let value_ptr = &mut self.tree.storage.get_unchecked_mut(
                        &parent_key
                    ).value;
                    ptr::write(value_ptr, NodeData::Leaf(payload));
                }
            }
        } else if Some(&self.key) == right_child.as_ref() {
            *right_child = None;
        } else { unsafe {
            // SAFETY: a node cannot have a parent which does not list it as one
            // of its children
            if cfg!(debug_assertions) {
                panic!("\
failed to identify whether the node is the left or right child");
            }
            hint::unreachable_unchecked()
        }}
        let key = self.key.clone();
        match self.tree.storage.remove(&key).value {
            NodeData::Leaf(x) => Ok(x),
            NodeData::Branch {..} => unsafe {
                // SAFETY: the outer match tests for self being a branch node
                hint::unreachable_unchecked()
            },
        }
    }
    /// Attempts to remove a branch node without using recursion. If its parent only had one child, it's replaced with a leaf node, keeping its original payload. *This method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node. The `try_remove_leaf`/`try_remove_leaf_with` methods exist for that.
    /// - The node was the root node, which can never be removed.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_branch(
        &mut self,
    ) -> Result<(D, D, Option<D>), TryRemoveBranchError> {
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
                if left_child_ref.is_branch()
                        || (right_child_ref.map(|x| x.is_branch()) == Some(true)) {
                    return Err(TryRemoveBranchError::HadBranchChild)
                }
            },
            NodeData::Leaf(..) => return Err(TryRemoveBranchError::WasLeafNode),
        }
        let parent_key = self.node()
            .parent
            .as_ref()
            .cloned()
            .ok_or(TryRemoveBranchError::WasRootNode)?;
        let (left_child, right_child, payload) = match unsafe {
            // SAFETY: parent key is guaranteed to be valid
            &mut self.tree.storage.get_unchecked_mut(&parent_key).value
        } {
            NodeData::Branch {left_child, right_child, payload} => (
                left_child, right_child, payload,
            ),
            NodeData::Leaf(..) => unsafe {
                // SAFETY: cannot have leaf node as parent
                hint::unreachable_unchecked()
            }
        };
        if &self.key == left_child {
            if let Some(right_child_ref) = right_child {
                mem::swap(left_child, right_child_ref);
                *right_child = None;
            } else {
                // Destroy the mutable references to modify parent
                drop((left_child, right_child));
                let payload = unsafe {
                    // SAFETY: we're overwriting the memory right afterwards, and
                    // the pointer comes from a reference and therefore is valid
                    ptr::read(payload)
                };
                unsafe {
                    // SAFETY: we're overwriting what we just read, which is
                    // still behind a reference and still valid
                    let value_ptr = &mut self.tree.storage.get_unchecked_mut(
                        &parent_key
                    ).value;
                    ptr::write(value_ptr, NodeData::Leaf(payload));
                }
            }
        } else if Some(&self.key) == right_child.as_ref() {
            *right_child = None;
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
            old_payload,
            left_child_key,
            right_child_key,
        ) = match self.tree.storage.remove(&key).value {
            NodeData::Branch {
                payload: old_payload,
                left_child: left_child_key,
                right_child: right_child_key,
            } => (
                old_payload,
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
        Ok((old_payload, left_child_payload, right_child_payload))
    }
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, keeping its original payload. *This method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_children(
        &mut self,
    ) -> Result<(D, Option<D>), TryRemoveChildrenError> {
        let (left_child_key, right_child_key) = match &self.node().value {
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
                if left_child_ref.is_branch()
                    || (right_child_ref.as_ref().map(NodeRef::is_branch) == Some(true)) {
                    return Err(TryRemoveChildrenError::HadBranchChild)
                }
                (left_child_ref.key, right_child_ref.map(|x| x.key))
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
        let own_key = &self.key;
        let payload = unsafe {
            // SAFETY: we're overwriting the memory right afterwards, and the pointer comes from a
            // reference and therefore is valid; the get_unchecked_mut is also safe because the
            // key of `self` is assumed to be valid
            ptr::read(match &self.tree.storage.get_unchecked(own_key).value {
                NodeData::Branch {payload, ..} => payload,
                NodeData::Leaf(..) => {
                    // SAFETY: we checked for `self` being a leaf node in the beginning
                    hint::unreachable_unchecked()
                },
            })
        };
        unsafe {
            // SAFETY: we're overwriting what we just read, which is
            // still behind a reference and still valid
            let value_ptr = &mut self.tree.storage.get_unchecked_mut(
                own_key,
            ).value;
            ptr::write(value_ptr, NodeData::Leaf(payload));
        }
        Ok((left_child_payload, right_child_payload))
    }
    // TODO try_remove_branch and try_remove_children
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