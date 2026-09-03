#![cfg(feature = "reactivity")]

use crate::extensions::ComponentColumn;
use parking_lot::RwLock;
use std::any::{Any, TypeId};

mod added;
mod changed;

#[derive(Debug)]
pub(crate) struct TrackedComponentMeta {
    pub(crate) component_id: TypeId,
    pub(crate) marker_id: TypeId,
    pub(crate) create_marker_column: fn() -> ComponentColumn,
    pub(crate) push_default_marker: unsafe fn(&mut ComponentColumn),
    pub(crate) clear_column_markers: unsafe fn(&mut dyn Any),
}

pub(crate) static TRACKED_COMPONENTS: RwLock<Vec<TrackedComponentMeta>> = RwLock::new(Vec::new());

pub use changed::{Changed, ChangedTracker};

pub(crate) use changed::{ChangedMarker, Mut};

pub use added::{Added, AddedTracker};
