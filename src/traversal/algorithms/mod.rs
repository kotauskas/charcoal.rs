//! Ubiquitous algorithms for trees.
//!
//! This includes:
//! - Recursive removal
//! - *More to come*

mod recursive_removal;
pub use recursive_removal::*;

use super::{
    VisitorMut,
    Traversable,
    TraversableMut,
    VisitorDirection,
    CursorResult,
    CursorDirectionError,
};
