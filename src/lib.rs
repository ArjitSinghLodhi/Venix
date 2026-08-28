pub mod app;
pub mod commands;
#[cfg(feature = "reactivity")]
pub mod detection;
pub mod entity;
pub mod events;
pub mod query;
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
    pub use crate::app::{
        app::App,
        plugin::{Plugin, PluginsBuildAll},
    };
    pub use crate::commands::{Commands, ParallelCommands, bundle::ComponentBundle};
    #[cfg(feature = "derive")]
    pub use crate::derive::*;
    #[cfg(feature = "reactivity")]
    pub use crate::detection::{added::*, changed::*};
    pub use crate::entity::Entity;
    pub use crate::events::*;
    pub use crate::query::{ergonomic_params::*, filter::*, query::*};
    pub use crate::resources::*;
    pub use crate::schedule::{DefaultSchedulesPlugin, schedule::*, schedules_list::*};
    pub use crate::system::validation::System;
    pub use crate::system::validation::SystemId;
    pub use crate::world::storage::World;
}

pub mod extensions {
    pub use crate::system::validation::{
        AccessHashSet, AccessVec, FunctionData, FunctionSystem, IntoSystem, IntoSystemConfigs,
        ParamAccess, System, SystemExt, SystemParam,
    };
    pub use crate::world::archetypes::{Archetype, ComponentColumn};
    pub use crate::world::storage::World;
}
