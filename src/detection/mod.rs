#[cfg(feature = "reactivity")]
use std::{
    any::{Any, TypeId},
    sync::RwLock,
};

#[cfg(feature = "reactivity")]
use crate::extensions::ComponentColumn;

#[cfg(feature = "reactivity")]
pub mod changed;

#[cfg(feature = "reactivity")]
pub mod added;

#[cfg(feature = "reactivity")]
#[derive(Debug)]
pub(crate) struct TrackedComponentMeta {
    pub(crate) component_id: TypeId,
    pub(crate) marker_id: TypeId,
    pub(crate) create_marker_column: fn() -> ComponentColumn,
    pub(crate) push_default_marker: unsafe fn(&mut ComponentColumn),
    pub(crate) clear_column_markers: unsafe fn(&mut dyn Any),
}

#[cfg(feature = "reactivity")]
pub(crate) static TRACKED_COMPONENTS: RwLock<Vec<TrackedComponentMeta>> = RwLock::new(Vec::new());
