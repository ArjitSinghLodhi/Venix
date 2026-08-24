pub mod app;
pub mod commands;
pub mod entity;
pub mod query;
mod registry;
pub mod resources;
pub mod schedule;
mod system;
mod world;

pub use fxhash;
pub use indexmap;
pub use rayon;
#[cfg(feature = "derive")]
pub mod derive {
    pub use venix_macros::ComponentBundle;
    pub use venix_macros::QueryFilter;
    pub use venix_macros::SystemParam;
    pub use venix_macros::QueryData;
}
pub mod prelude {
    pub use crate::app::{
        app::App,
        plugin::{Plugin, PluginsBuildAll},
    };
    pub use crate::commands::{
        bundle::ComponentBundle,
        {commands::Commands, parallel_commands::ParallelCommands},
    };
    #[cfg(feature = "derive")]
    pub use crate::derive::*;
    pub use crate::entity::Entity;
    pub use crate::query::{changed::*, ergonomic_params::*, filter::*, query::*};
    pub use crate::resources::*;
    pub use crate::schedule::{DefaultSchedulesPlugin, schedule::*, schedules_list::*};
    pub use crate::system::validation::System;
    pub use crate::world::storage::World;
}

pub mod extensions {
    pub use crate::system::validation::{
        AccessHashSet, AccessVec, FunctionData, FunctionSystem, IntoSystem, IntoSystemConfigs,
        ParamAccess, System, SystemParam,
    };
    pub use crate::world::archetypes::{Archetype, ComponentColumn};
    pub use crate::world::storage::World;
}
