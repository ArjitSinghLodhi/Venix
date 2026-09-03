//! # Venix Engine
//!
//! ## Core Architecture Invariants
//!
//! To maintain absolute safety and high performance, Venix enforces strict operational rules:
//!
//! * **Entity Despawn Invariant:** All cloned handles referencing an entity must be completely dropped
//!   before that entity's scheduled despawn command is executed. Violating this triggers an explicit runtime
//!   panic displaying the archetype's components. See [`DefaultSchedulesPlugin`] for how the built-in
//!   schedules are designed to help with this, and read [`Commands::despawn()`] for more information.
//!
//! [`DefaultSchedulesPlugin`]: crate::schedule::DefaultSchedulesPlugin
//! [`Commands::despawn()`]: crate::commands::Commands::despawn

#![allow(clippy::module_inception)]

pub mod app;
pub mod commands;
pub mod entity;
pub mod events;
pub mod query;
#[cfg(feature = "reactivity")]
pub mod reactivity;
mod registry;
pub mod resources;
pub mod schedule;
pub mod system;
mod world;

pub use fxhash;
pub use indexmap;
pub use rayon;
#[cfg(feature = "derive")]
pub mod derive {
    pub use venix_macros::ComponentBundle;
    pub use venix_macros::QueryData;
    pub use venix_macros::QueryFilter;
    pub use venix_macros::SystemParam;
}
pub mod prelude {
    pub use crate::app::{App, Plugin, PluginsBuildAll};
    pub use crate::commands::{Commands, ParallelCommands, bundle::ComponentBundle};
    #[cfg(feature = "derive")]
    pub use crate::derive::*;
    pub use crate::entity::Entity;
    #[cfg(feature = "events")]
    pub use crate::events::*;
    pub use crate::query::*;
    #[cfg(feature = "reactivity")]
    pub use crate::reactivity::*;
    pub use crate::resources::*;
    pub use crate::schedule::{DefaultSchedulesPlugin, schedule::*, schedules_list::*};
    pub use crate::system::validation::System;
    pub use crate::world::storage::World;
}

pub mod extensions {
    pub use crate::system::validation::{
        AccessHashSet, AccessVec, FunctionData, FunctionSystem, IntoSystem, IntoSystemConfigs,
        ParamAccess, System, SystemData, SystemExt, SystemParam,
    };
    pub use crate::world::archetypes::{Archetype, ComponentColumn};
    pub use crate::world::storage::World;
}
