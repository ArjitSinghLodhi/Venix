pub mod app;
pub mod commands;
pub mod entity;
pub mod query;
mod registry;
pub mod schedule;
mod system;
mod world;
pub use rayon;
pub mod resources;

pub mod prelude {
    pub use crate::app::{
        app::App,
        plugin::{Plugin, PluginsBuildAll},
    };
    pub use crate::commands::{commands::Commands, parallel_commands::ParallelCommands};
    pub use crate::entity::Entity;
    pub use crate::query::{changed::*, ergonomic_params::*, filter::*, query::*};
    pub use crate::resources::*;
    pub use crate::schedule::{DefaultSchedulesPlugin, schedule::*, schedules_list::*};
    pub use crate::system::validation::System;
    pub use crate::world::storage::World;
}

pub mod extensions {
    pub use crate::system::validation::{
        FunctionData, FunctionSystem, IntoSystem, IntoSystemConfigs, ParamAccess, System,
        SystemParam,
    };
    pub use crate::world::archetypes::{
        AnyColumn, Archetype, ArchetypeId, ArchetypeManager, ComponentColumn,
    };
    pub use crate::world::storage::World;
}
