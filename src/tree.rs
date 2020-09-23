/// Tree-like data structures.
pub trait Tree {
    /// The data payload of leaf nodes.
    type Leaf;
    /// The data payload of branch nodes.
    type Branch;
    /// The key used to address the tree's contents.
    ///
    /// Keys are not guaranteed to be stable — relying on their stability for memory safety might cause undefined behavior. The only case in which those keys should actually be stored for extended periods of time is when a visitor needs to remember node locations, since it's a logic error to interleave `step`/`step_mut` calls for read-only and mutating visitors or for multiple mutating visitors; still, visitors should check for key error conditions and panic if those happen.
    type Key;

    /// Creates a tree with the specified root node.
    fn new(root: Self::Leaf) -> Self;
}