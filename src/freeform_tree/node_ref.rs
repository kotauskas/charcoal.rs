use core::{ptr, mem, fmt::Debug, hint, convert, iter::FusedIterator};
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
use super::{TryPushError, FreeformTree, Node, NodeData};

// A reference to a node in a freeform tree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree.
#[derive(Debug)]
pub struct NodeRef<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    tree: &'a FreeformTree<B, L, K, S>,
    key: K,
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

/// A *mutable* reference to a node in a freeform tree.
///
/// Since this type does not point to the node directly, but rather the tree the node is in and the key of the node in the storage, it can be used to traverse the tree and modify it as a whole.
#[derive(Debug)]
pub struct NodeRefMut<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    tree: &'a mut FreeformTree<B, L, K, S>,
    key: K,
}
impl<'a, B, L, K, S> NodeRefMut<'a, B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a new `NodeRefMut` pointing to the specified key in the storage, or `None` if it does not exist.
    pub fn new_raw(tree: &'a mut FreeformTree<B, L, K, S>, key: K) -> Option<Self> {
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
    pub unsafe fn new_raw_unchecked(tree: &'a mut FreeformTree<B, L, K, S>, key: K) -> Self {
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
    /// Downgrades a mutable reference to an immutable one.
    pub fn downgrade(self) -> NodeRef<'a, B, L, K, S> {
        unsafe {
            // SAFETY: validity gurantees are equal for NodeRef and NodeRefMut
            NodeRef::new_raw_unchecked(self.tree, self.key)
        }
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
    /// Returns a reference to the sibling of the pointee which comes before it in order, or `None` if it's the first child of its parent.
    pub fn prev_sibling(&self) -> Option<NodeRef<'_, B, L, K, S>> {
        self.node().next_sibling.as_ref().map(|x| unsafe {
            // SAFETY: next sibling key is always valid
            NodeRef::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns a *mutable* reference to the sibling of the pointee which comes before it in order, or `None` if it's the first child of its parent.
    pub fn prev_sibling_mut(&mut self) -> Option<NodeRefMut<'_, B, L, K, S>> {
        let key = self.node().prev_sibling.as_ref().cloned();
        key.map(move |x| unsafe {
            // SAFETY: as above
            Self::new_raw_unchecked(self.tree, x)
        })
    }
    /// Returns a reference to the sibling of the pointee which comes after it in order, or `None` if it's the last child of its parent.
    pub fn next_sibling(&self) -> Option<NodeRef<'_, B, L, K, S>> {
        self.node().next_sibling.as_ref().map(|x| unsafe {
            // SAFETY: next sibling key is always valid
            NodeRef::new_raw_unchecked(self.tree, x.clone())
        })
    }
    /// Returns a *mutable* reference to the sibling of the pointee which comes after it in order, or `None` if it's the last child of its parent.
    ///
    /// This is the only way to iterate through the siblings of a node with mutable access without extra allocations. In the future, a more ergonomic interface might become available.
    pub fn next_sibling_mut(&mut self) -> Option<NodeRefMut<'_, B, L, K, S>> {
        let key = self.node().next_sibling.as_ref().cloned();
        key.map(move |x| unsafe {
            // SAFETY: as above
            Self::new_raw_unchecked(self.tree, x)
        })
    }
    /// Returns a reference to the first child of the node, or `None` if it's a leaf node.
    pub fn first_child(&self) -> Option<NodeRef<'_, B, L, K, S>> {
        if let NodeData::Branch { first_child, .. } = &self.node().value {
            unsafe {
                // SAFETY: first child key is always valid
                Some(NodeRef::new_raw_unchecked(self.tree, first_child.clone()))
            }
        } else {
            None
        }
    }
    /// Returns a *mutable* reference to the first child of the node, or `None` if it's a leaf node.
    pub fn first_child_mut(&mut self) -> Option<NodeRefMut<'_, B, L, K, S>> {
        if let NodeData::Branch { first_child, .. } = &self.node().value {
            let key = first_child.clone();
            unsafe {
                // SAFETY: first child key is always valid
                Some(NodeRefMut::new_raw_unchecked(self.tree, key))
            }
        } else {
            None
        }
    }
    /// Returns a reference to the last child of the node, or `None` if it's a leaf node.
    pub fn last_child(&self) -> Option<NodeRef<'_, B, L, K, S>> {
        if let NodeData::Branch { last_child, .. } = &self.node().value {
            let key = last_child.clone();
            unsafe {
                // SAFETY: last child key is always valid
                Some(NodeRef::new_raw_unchecked(self.tree, key))
            }
        } else {
            None
        }
    }
    /// Returns a *mutable* reference to the last child of the node, or `None` if it's a leaf node.
    pub fn last_child_mut(&mut self) -> Option<NodeRefMut<'_, B, L, K, S>> {
        if let NodeData::Branch { last_child, .. } = &self.node().value {
            let key = last_child.clone();
            unsafe {
                // SAFETY: last child key is always valid
                Some(NodeRefMut::new_raw_unchecked(self.tree, key))
            }
        } else {
            None
        }
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
    /// Returns `true` if the node is a *branch*, i.e. has one or more child nodes; `false` otherwise.
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
    /// Returns a *mutable* reference to the data stored in the node.
    pub fn value_mut(&mut self) -> NodeValue<&'_ mut B, &'_ mut L> {
        self.node_mut().value.as_mut().into_value()
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
    /// Returns an iterator over references to the children of the node, or `None` if the node is a leaf node.
    pub fn children(&self) -> Option<NodeChildrenIter<'_, B, L, K, S>> {
        self.first_child().map(NodeRef::siblings)
    }
    /// Returns an iterator over the raw keys of the children of the node, or `None` if the node is a leaf node.
    pub fn children_keys(&self) -> Option<NodeChildKeysIter<'_, B, L, K, S>> {
        self.first_child().map(NodeRef::sibling_keys)
    }

    /// Converts a leaf node into a branch node with the specified leaf children, using the provided closure to convert the payload.
    ///
    /// # Errors
    /// Will fail if the node is already a branch node. In such a case, the provided values for the children are returned back to the caller.
    pub fn make_branch_with<I: IntoIterator<Item = L>>(
        &mut self,
        children: I,
        f: impl FnOnce(L) -> B,
    ) -> Result<(), MakeBranchError<L, I>> {
        // The borrow checker forced us into doing a two-stage thing here
        if self.is_branch() {
            return Err(MakeBranchError {
                packed_children: children,
            });
        }
        let mut children = children.into_iter();
        let first_element = if let Some(x) = children.next() {
            self.tree.storage.add(unsafe {
                // SAFETY: parent key validity guaranteed via own key validity guarantee
                Node::leaf(x, None, None, Some(self.key.clone()))
            })
        } else {
            return Ok(());
        };
        let old_payload_ref = if let NodeData::Leaf(val) = &self.node().value {
            val
        } else {
            unsafe {
                // SAFETY: We checked for this in the beginning of the function
                hint::unreachable_unchecked()
            }
        };
        let old_payload = unsafe {
            // SAFETY: we're overwriting this afterwards
            ptr::read(old_payload_ref)
        };
        let new_payload = f(old_payload);
        unsafe {
            // SAFETY: as above
            ptr::write(
                &mut self.node_mut().value,
                NodeData::Branch {
                    payload: new_payload,
                    first_child: first_element.clone(),
                    last_child: first_element.clone(),
                },
            )
        }
        let mut current_element_key = first_element;
        let mut previous_element_key = None;
        for next_element in children {
            let next_element_key = self.tree.storage.add(unsafe {
                // SAFETY: see safety for first_element
                Node::leaf(
                    next_element,
                    previous_element_key,
                    None,
                    Some(self.key.clone()),
                )
            });
            let next_sibling_key_ref = unsafe {
                // SAFETY: key validity gurantee comes from safety contract of Storage
                &mut self
                    .tree
                    .storage
                    .get_unchecked_mut(&current_element_key)
                    .next_sibling
            };
            *next_sibling_key_ref = Some(next_element_key.clone());
            // Move the old current element to previous, put the next one into the current
            previous_element_key = Some(mem::replace(&mut current_element_key, next_element_key));
        }
        match &mut self.node_mut().value {
            NodeData::Branch { last_child, .. } => {
                // Update the last child key to point to the last one we added.
                *last_child = current_element_key;
            }
            NodeData::Leaf(..) => unsafe {
                // SAFETY: the method makes numerous checks for a leaf node
                hint::unreachable_unchecked()
            },
        }
        Ok(())
    }

    /// Adds a child node to the node's children set after all other ones, failing if it's not a branch node.
    ///
    /// # Errors
    /// Will fail only if the node was a leaf node before the operation. The same operation could be retried with [`push_back_with`]/[`push_front_with`], or [`push_back`]/[`push_front`] if the same type is used for leaf node and branch node payloads.
    ///
    /// [`push_back_with`]: struct.NodeRefMut.html#method.push_back_with " "
    /// [`push_front_with`]: struct.NodeRefMut.html#method.push_front_with " "
    /// [`push_back`]: struct.NodeRefMut.html#method.push_back " "
    /// [`push_front`]: struct.NodeRefMut.html#method.push_front " "
    pub fn try_push_back(&mut self, child_payload: L) -> Result<(), TryPushError<L>> {
        if self.is_leaf() {
            return Err(TryPushError { child_payload });
        }
        let child_key = self.tree.storage.add(unsafe {
            // SAFETY: key validity guaranteed
            Node::leaf(child_payload, None, None, Some(self.key.clone()))
        });
        let old_last_child_key_ref = match &mut self.node_mut().value {
            NodeData::Branch { last_child, .. } => last_child,
            NodeData::Leaf(..) => unsafe {
                // SAFETY: we did a leaf check in the beginning
                hint::unreachable_unchecked()
            },
        };
        let old_last_child_key = mem::replace(old_last_child_key_ref, child_key.clone());
        let old_last_child = unsafe {
            // SAFETY: key validity guarantee
            self.tree.storage.get_unchecked_mut(&old_last_child_key)
        };
        old_last_child.next_sibling = Some(child_key.clone());
        let new_last_child = unsafe {
            // SAFETY: as above
            self.tree.storage.get_unchecked_mut(&child_key)
        };
        new_last_child.prev_sibling = Some(old_last_child_key);
        Ok(())
    }
    /// Adds a child node to the node's children set before all other ones, failing if it's not a branch node.
    ///
    /// # Errors
    /// Will fail only if the node was a leaf node before the operation. The same operation could be retried with [`push_back_with`]/[`push_front_with`], or [`push_back`]/[`push_front`] if the same type is used for leaf node and branch node payloads.
    ///
    /// [`push_back_with`]: struct.NodeRefMut.html#method.push_back_with " "
    /// [`push_front_with`]: struct.NodeRefMut.html#method.push_front_with " "
    /// [`push_back`]: struct.NodeRefMut.html#method.push_back " "
    /// [`push_front`]: struct.NodeRefMut.html#method.push_front " "
    pub fn try_push_front(&mut self, child_payload: L) -> Result<(), TryPushError<L>> {
        if self.is_leaf() {
            return Err(TryPushError { child_payload });
        }
        let child_key = self.tree.storage.add(unsafe {
            // SAFETY: key validity guaranteed
            Node::leaf(child_payload, None, None, Some(self.key.clone()))
        });
        let old_first_child_key_ref = match &mut self.node_mut().value {
            NodeData::Branch { first_child, .. } => first_child,
            NodeData::Leaf(..) => unsafe {
                // SAFETY: we did a leaf check in the beginning
                hint::unreachable_unchecked()
            },
        };
        let old_first_child_key = mem::replace(old_first_child_key_ref, child_key.clone());
        let old_first_child = unsafe {
            // SAFETY: key validity guarantee
            self.tree.storage.get_unchecked_mut(&old_first_child_key)
        };
        old_first_child.prev_sibling = Some(child_key.clone());
        let new_first_child = unsafe {
            // SAFETY: as above
            self.tree.storage.get_unchecked_mut(&child_key)
        };
        new_first_child.next_sibling = Some(old_first_child_key);
        Ok(())
    }

    /// Attempts to remove a leaf node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure (the previous value is passed into the closure).
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a branch node, which would require recursion to remove, and this function explicitly does not implement recursive removal.
    /// - The node was the root node, which can never be removed.
    pub fn try_remove_leaf_with(
        self,
        branch_to_leaf: impl FnOnce(B) -> L,
    ) -> Result<L, TryRemoveLeafError> {
        if !self.is_leaf() {
            return Err(TryRemoveLeafError::WasBranchNode);
        }
        let parent_key = self
            .node()
            .parent
            .as_ref()
            .cloned()
            .ok_or(TryRemoveLeafError::WasRootNode)?;
        let (prev_sibling_key, next_sibling_key) = (
            self.node().prev_sibling.clone(),
            self.node().next_sibling.clone(),
        );
        if let Some(prev_sibling_key) = &prev_sibling_key {
            let prev_sibling = unsafe {
                // SAFETY: key validity guarantee
                self.tree.storage.get_unchecked_mut(prev_sibling_key)
            };
            // Will use the next sibling or None to indicate that there is no next sibling
            prev_sibling.next_sibling = next_sibling_key.clone();
        } else {
            // No previous sibling key means that we're the first child of our
            // parent, so fix the first child key value
            let parent = unsafe {
                // SAFETY: as above
                self.tree.storage.get_unchecked_mut(&parent_key)
            };
            if let NodeData::Branch { first_child, .. } = &mut parent.value {
                *first_child = self.key.clone();
            } else {
                unsafe {
                    unreachable_debugchecked("parent nodes cannot be leaves");
                }
            }
        }
        if let Some(next_sibling_key) = &next_sibling_key {
            let next_sibling = unsafe {
                // SAFETY: as above
                self.tree.storage.get_unchecked_mut(next_sibling_key)
            };
            // Similar thing here
            next_sibling.prev_sibling = prev_sibling_key.clone();
        } else {
            // No next sibling key means that we're the last child of our
            // parent, so fix the last child key value
            let parent = unsafe {
                // SAFETY: as above
                self.tree.storage.get_unchecked_mut(&parent_key)
            };
            if let NodeData::Branch { last_child, .. } = &mut parent.value {
                *last_child = self.key.clone();
            } else {
                unsafe {
                    unreachable_debugchecked("parent nodes cannot be leaves");
                }
            }
        }
        if prev_sibling_key.is_none() && next_sibling_key.is_none() {
            let parent = unsafe {
                // SAFETY: as above
                self.tree.storage.get_unchecked_mut(&parent_key)
            };
            let parent_payload_ref = if let NodeData::Branch { payload, .. } = &parent.value {
                payload
            } else {
                unsafe { unreachable_debugchecked("parent nodes cannot be leaves") }
            };
            let parent_payload = unsafe {
                // SAFETY: we're overwriting this afterwards
                ptr::read(parent_payload_ref)
            };
            let new_parent_payload = abort_on_panic(|| branch_to_leaf(parent_payload));
            unsafe {
                // SAFETY: see read()
                ptr::write(&mut parent.value, NodeData::Leaf(new_parent_payload));
            }
        }
        let val = self.tree.storage.remove(&self.key);
        if let NodeData::Leaf(val) = val.value {
            Ok(val)
        } else {
            unsafe {
                // SAFETY: we checked for a branch node in the beginning
                hint::unreachable_unchecked()
            }
        }
    }
    /// Attempts to remove a branch node without using recursion. If its parent only had one child, it's replaced with a leaf node, the value for which is provided by the specified closure (the previous value is passed into the closure). The children which this branch node had are fed into the second closure.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node. The `try_remove_leaf`/`try_remove_leaf_with` methods exist for that.
    /// - The node was the root node, which can never be removed.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_branch_with(
        self,
        branch_to_leaf: impl FnOnce(B) -> L,
        mut collector: impl FnMut(L),
    ) -> Result<B, TryRemoveBranchError> {
        if !self.is_branch() {
            return Err(TryRemoveBranchError::WasLeafNode);
        }
        let parent_key = if let Some(parent) = &self.node().parent {
            parent.clone()
        } else {
            return Err(TryRemoveBranchError::WasRootNode);
        };
        let first_child_key = if let NodeData::Branch { first_child, .. } = &self.node().value {
            first_child.clone()
        } else {
            unsafe {
                // SAFETY: we checked for a leaf node in the beginning
                hint::unreachable_unchecked()
            }
        };
        // FIXME this requires double iteration for checking, maybe we can
        // store the count of branch children in the parent
        let branch_child = self
            .children()
            .unwrap_or_else(|| unsafe {
                // SAFETY: we checked for a leaf node in the beginning
                hint::unreachable_unchecked()
            })
            .zip(0_u32..)
            .find(|x| x.0.is_branch());
        if let Some((_, branch_child_index)) = branch_child {
            return Err(TryRemoveBranchError::HadBranchChild(branch_child_index));
        }
        let mut current_child_key = first_child_key;
        loop {
            let next_child_key = unsafe {
                // SAFETY: key validity guarantee
                self.tree
                    .storage
                    .get_unchecked(&current_child_key)
                    .next_sibling
                    .clone()
            };
            let current_child = {
                let val = self.tree.storage.remove(&current_child_key);
                if let NodeData::Leaf(val) = val.value {
                    val
                } else {
                    unsafe {
                        // SAFETY: we checked for branch children before
                        hint::unreachable_unchecked()
                    }
                }
            };
            abort_on_panic(|| collector(current_child));
            current_child_key = if let Some(next_child_key) = next_child_key {
                next_child_key
            } else {
                break;
            };
        }
        let parent_ref = unsafe {
            // SAFETY: key validity guarantee
            self.tree.storage.get_unchecked(&parent_key)
        };
        let is_only_sibling = if let NodeData::Branch {
            first_child,
            last_child,
            ..
        } = &parent_ref.value
        {
            first_child == last_child
        } else {
            unsafe { unreachable_debugchecked("parent nodes cannot be leaves") }
        };
        if is_only_sibling {
            let old_parent_payload_ref = if let Some(NodeData::Branch { payload, .. }) =
                self.parent().map(|x| &x.node().value)
            {
                payload
            } else {
                unsafe {
                    // SAFETY: we checked for a root node before
                    hint::unreachable_unchecked()
                }
            };
            let old_parent_payload = unsafe {
                // SAFETY: we are overwriting this afterwards
                ptr::read(old_parent_payload_ref)
            };
            let parent = unsafe {
                // SAFETY: key validity guarantee
                self.tree.storage.get_unchecked_mut(&parent_key)
            };
            unsafe {
                ptr::write(
                    &mut parent.value,
                    NodeData::Leaf(abort_on_panic(|| branch_to_leaf(old_parent_payload))),
                )
            };
        }
        if let NodeData::Branch { payload, .. } = self.tree.storage.remove(&self.key).value {
            Ok(payload)
        } else {
            unsafe {
                // SAFETY: we checked for a leaf node in the beginning
                hint::unreachable_unchecked()
            }
        }
    }
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, the value for which is provided by the specified closure. Another closure is used to collect all removed children.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_children_with(
        &mut self,
        branch_to_leaf: impl FnOnce(B) -> L,
        mut collector: impl FnMut(L),
    ) -> Result<(), TryRemoveChildrenError> {
        if !self.is_branch() {
            return Err(TryRemoveChildrenError::WasLeafNode);
        }
        let first_child_key = if let NodeData::Branch { first_child, .. } = &self.node().value {
            first_child.clone()
        } else {
            unsafe {
                // SAFETY: we checked for a leaf node in the beginning
                hint::unreachable_unchecked()
            }
        };
        let branch_child = self
            .children()
            .unwrap_or_else(|| unsafe {
                // SAFETY: we checked for a leaf node in the beginning
                hint::unreachable_unchecked()
            })
            .zip(0_u32..)
            .find(|x| x.0.is_branch());
        if let Some((_, branch_child_index)) = branch_child {
            return Err(TryRemoveChildrenError::HadBranchChild(branch_child_index));
        }
        let mut current_child_key = first_child_key;
        loop {
            let next_child_key = unsafe {
                // SAFETY: key validity guarantee
                self.tree
                    .storage
                    .get_unchecked(&current_child_key)
                    .next_sibling
                    .clone()
            };
            let current_child = {
                let val = self.tree.storage.remove(&current_child_key);
                if let NodeData::Leaf(val) = val.value {
                    val
                } else {
                    unsafe {
                        // SAFETY: we checked for branch children before
                        hint::unreachable_unchecked()
                    }
                }
            };
            abort_on_panic(|| collector(current_child));
            current_child_key = if let Some(next_child_key) = next_child_key {
                next_child_key
            } else {
                break;
            };
        }
        let old_payload_ref = if let NodeData::Branch { payload, .. } = &self.node().value {
            payload
        } else {
            unsafe {
                // SAFETY: we checked for a leaf node before
                hint::unreachable_unchecked()
            }
        };
        let old_payload = unsafe {
            // SAFETY: we are overwriting this afterwards
            ptr::read(old_payload_ref)
        };
        unsafe {
            ptr::write(
                &mut self.node_mut().value,
                NodeData::Leaf(abort_on_panic(|| branch_to_leaf(old_payload))),
            )
        };
        Ok(())
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
            // SAFETY: all existing NodeRefMuts are guaranteed to not be dangling
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
    pub fn make_branch<I: IntoIterator<Item = D>>(
        &mut self,
        children: I,
    ) -> Result<(), MakeBranchError<D, I>> {
        self.make_branch_with(children, convert::identity)
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
    /// Attempts to remove a branch node without using recursion. The children of the node are fed into the specified closure. If its parent only had one child, it's replaced with a leaf node, keeping its original payload, which is why *this method is only available when the payload for leaf nodes and branch nodes is the same.*
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node. The `try_remove_leaf`/`try_remove_leaf_with` methods exist for that.
    /// - The node was the root node, which can never be removed.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_branch(self, collector: impl FnMut(D)) -> Result<D, TryRemoveBranchError> {
        self.try_remove_branch_with(convert::identity, collector)
    }
    /// Attempts to remove a branch node's children without using recursion, replacing it with a leaf node, keeping its original payload. Because of that, *this method is only available when the payload for leaf nodes and branch nodes is the same.* Removed children are fed into the specified closure.
    ///
    /// # Errors
    /// Will fail in the following scenarios:
    /// - The node was a leaf node, which cannot have children by definition.
    /// - One or more of the node's children were a branch node, which thus would require recursion to remove.
    pub fn try_remove_children(
        &mut self,
        collector: impl FnMut(D),
    ) -> Result<(), TryRemoveChildrenError> {
        self.try_remove_children_with(convert::identity, collector)
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

/// An iterator over keys of the siblings of a freeform tree node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeSiblingKeysIter<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    tree: &'a FreeformTree<B, L, K, S>,
    key: Option<K>,
}
/// An iterator over keys of the children of a freeform tree node.
type NodeChildKeysIter<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>> =
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
    NodeSiblingKeysIter<'a, B, L, K, S>,
)
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq;
/// An iterator over references to the children of a freeform tree node.
type NodeChildrenIter<'a, B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>> =
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
