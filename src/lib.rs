pub mod app;
pub mod commands;
pub mod entity;
pub mod query;
mod registry;
pub mod schedule;
mod system;
pub mod world;
pub use rayon;

pub mod prelude {
    pub use crate::app::app::App;
    pub use crate::app::plugin::Plugin;
    pub use crate::app::plugin::PluginsBuildAll;
    pub use crate::commands::commands::Commands;
    pub use crate::entity::Entity;
    pub use crate::query::changed::Changed;
    pub use crate::query::ergonomic_params::*;
    pub use crate::query::filter::*;
    pub use crate::query::query::*;
    pub use crate::query::resources::*;
    pub use crate::schedule::DefaultSchedulesPlugin;
    pub use crate::schedule::schedule::*;
    pub use crate::schedule::schedules_list::*;
    pub use crate::system::validation::System;
    pub use crate::world::storage::World;
}

pub mod extensions {
    pub use crate::system::validation::IntoSystem;
    pub use crate::system::validation::IntoSystemConfigs;
    pub use crate::system::validation::ParamAccess;
    pub use crate::system::validation::SystemParam;
}
