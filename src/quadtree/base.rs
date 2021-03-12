use core::fmt::Debug;
use crate::{
    storage::{Storage, ListStorage, DefaultStorage, SparseStorage, SparseStorageSlot},
};
use super::{Node, NodeRef, NodeRefMut};

/// A quadtree.
///
/// See the [module-level documentation] for more.
///
/// [module-level documentation]: index.html " "
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Quadtree<B, L = B, K = usize, S = DefaultStorage<Node<B, L, K>>>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    pub(super) storage: S,
    pub(super) root: K,
}
impl<B, L, K, S> Quadtree<B, L, K, S>
where
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    /// Creates a quadtree with the specified value for the root node.
    ///
    /// # Example
    /// ```rust
    /// # use charcoal::Quadtree;
    /// // The only way to create a tree...
    /// let tree = Quadtree::<_>::new(87);
    /// // ...is to simply create the root leaf node and storage. The turbofish there is needed to
    /// // state that we are using the default storage method instead of asking the compiler to
    /// // infer it, which would be impossible.
    ///
    /// // No other nodes have been created yet:
    /// assert!(tree.root().is_leaf());
    /// ```
    pub fn new(root: L) -> Self {
        let mut storage = S::new();
        let root = storage.add(unsafe {
            // SAFETY: there isn't a root there yet
            Node::root(root)
        });
        Self { storage, root }
    }
    /// Creates a quadtree with the specified capacity for the storage.
    ///
    /// # Panics
    /// The storage may panic if it has fixed capacity and the specified value does not match it.
    ///
    /// # Example
    /// ```rust
    /// # use charcoal::Quadtree;
    /// // Let's create a tree, but with some preallocated space for more nodes:
    /// let mut tree = Quadtree::<_>::with_capacity(5, "Variable Names");
    /// // The turbofish there is needed to state that we are using the default storage method
    /// // instead of asking the compiler to infer it, which would be impossible.
    ///
    /// // Capacity does not affect the actual nodes:
    /// assert!(tree.root().is_leaf());
    ///
    /// // Not until we create them ourselves:
    /// tree.root_mut().make_branch([
    ///     "Foo", "Bar", "Baz", "Quux",
    /// ]);
    ///
    /// // If the default storage is backed by a dynamic memory allocation,
    /// // at most one has happened to this point.
    /// ```
    pub fn with_capacity(capacity: usize, root: L) -> Self {
        let mut storage = S::with_capacity(capacity);
        let root = storage.add(unsafe {
            // SAFETY: as above
            Node::root(root)
        });
        Self { storage, root }
    }

    /// Returns a reference to the root node of the tree.
    ///
    /// # Example
    /// ```rust
    /// # use charcoal::Quadtree;
    /// // A tree always has a root node:
    /// let tree = Quadtree::<_>::new("Root");
    ///
    /// assert_eq!(
    ///     // The into_inner() call extracts data from a NodeValue, which is used to generalize
    ///     // tres to both work with same and different types for payloads of leaf and branch
    ///     // nodes.
    ///     *tree.root().value().into_inner(),
    ///     "Root",
    /// );
    /// ```
    #[allow(clippy::missing_const_for_fn)] // there cannot be constant trees just yet
    pub fn root(&self) -> NodeRef<'_, B, L, K, S> {
        unsafe {
            // SAFETY: binary trees cannot be created without a root
            NodeRef::new_raw_unchecked(self, self.root.clone())
        }
    }
    /// Returns a *mutable* reference to the root node of the tree, allowing modifications to the entire tree.
    ///
    /// # Example
    /// ```rust
    /// # use charcoal::Quadtree;
    /// // A tree always has a root node:
    /// let mut tree = Quadtree::<_>::new("Root");
    ///
    /// let mut root_mut = tree.root_mut();
    /// // The into_inner() call extracts data from a NodeValue, which is used to generalize trees
    /// // to both work with same and different types for payloads of leaf and branch nodes.
    /// *(root_mut.value_mut().into_inner()) = "The Source of the Beer";
    /// ```
    pub fn root_mut(&mut self) -> NodeRefMut<'_, B, L, K, S> {
        unsafe {
            // SAFETY: as above
            NodeRefMut::new_raw_unchecked(self, self.root.clone())
        }
    }
}
impl<B, L, S> Quadtree<B, L, usize, SparseStorage<Node<B, L, usize>, S>>
where
    S: ListStorage<Element = SparseStorageSlot<Node<B, L, usize>>>,
{
    /// Removes all holes from the sparse storage.
    ///
    /// Under the hood, this uses `defragment_and_fix`. It's not possible to defragment without fixing the indicies, as that might cause undefined behavior.
    ///
    /// # Example
    /// ```rust
    /// use charcoal::quadtree::SparseVecQuadtree;
    ///
    /// // Create a tree which explicitly uses sparse storage:
    /// let mut tree = SparseVecQuadtree::new(0);
    /// // This is already the default, but for the sake of this example we'll stay explicit.
    ///
    /// // Add some elements for the holes to appear:
    /// tree.root_mut().make_branch([
    ///     1, 2, 3, 4,
    /// ]).unwrap(); // You can replace this with proper error handling
    /// tree
    ///     .root_mut()
    ///     .nth_child_mut(0)
    ///     .unwrap() // This too
    ///     .make_branch([5, 6, 7, 8])
    ///     .unwrap(); // And this
    ///
    /// tree
    ///     .root_mut()
    ///     .nth_child_mut(0)
    ///     .unwrap() // Same as above
    ///     .try_remove_children()
    ///     .unwrap(); // Same here
    ///
    /// // We ended up creating 4 holes:
    /// assert_eq!(tree.num_holes(), 4);
    /// // Let's patch them:
    /// tree.defragment();
    /// // Now there are none:
    /// assert_eq!(tree.num_holes(), 0);
    /// ```
    pub fn defragment(&mut self) {
        self.storage.defragment_and_fix()
    }
    /// Returns the number of holes in the storage. This operation returns immediately instead of looping through the entire storage, since the sparse storage automatically tracks the number of holes it creates and destroys.
    ///
    /// # Example
    /// See the example in [`defragment`].
    ///
    /// [`defragment`]: #method.defragment " "
    pub fn num_holes(&self) -> usize {
        self.storage.num_holes()
    }
    /// Returns `true` if there are no holes in the storage, `false` otherwise. This operation returns immediately instead of looping through the entire storage, since the sparse storage automatically tracks the number of holes it creates and destroys.
    ///
    /// # Example
    /// See the example in [`defragment`].
    ///
    /// [`defragment`]: #method.defragment " "
    pub fn is_dense(&self) -> bool {
        self.storage.is_dense()
    }
}
impl<B, L, K, S> Default for Quadtree<B, L, K, S>
where
    L: Default,
    S: Storage<Element = Node<B, L, K>, Key = K>,
    K: Clone + Debug + Eq,
{
    fn default() -> Self {
        Self::new(L::default())
    }
}
