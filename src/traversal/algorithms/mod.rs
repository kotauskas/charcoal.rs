//! Ubiquitous algorithms for trees.
//!
//! For now, this only includes recursive removal.

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
