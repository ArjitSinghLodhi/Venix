use crate::{
    app::{App, Plugin},
    schedule::schedules_list::{
        ApplyCommands, CleanupHandles, First, Last, PostUpdate, PreUpdate, Update,
    },
};

pub mod schedule;
pub mod schedules_list;

/// A plugin that registers the default execution schedules:
/// `First`, `PreUpdate`, `Update`, `PostUpdate`, `CleanupHandles`, `ApplyCommands`, and `Last`.
///
/// Among these, the `CleanupHandles` and `ApplyCommands` schedules serve special purposes:
///
/// * **`CleanupHandles`**: Queue commands (such as spawning and adding components) are applied
///   immediately before and after the systems registered in this schedule run. Despawn commands
///   are **not** applied yet.
/// * **`ApplyCommands`**: Systems in this schedule run, followed by the execution of
///   queue commands, and finally, despawn commands are processed.
///
/// This sequence is a deliberate mechanism designed to facilitate the use of the
/// [`Commands::despawn_iter()`] and [`Commands::will_despawn()`] functions. It ensures adherence
/// to Venix's strict rule: *All cloned handles referencing an entity must be dropped before
/// the entity's despawn command is applied.*
///
/// [`Commands::despawn_iter()`]: crate::commands::Commands::despawn_iter
/// [`Commands::will_despawn()`]: crate::commands::Commands::will_despawn
pub struct DefaultSchedulesPlugin;

impl Plugin for DefaultSchedulesPlugin {
    fn build(self, app: &mut App) {
        app.add_schedule(First)
            .add_schedule(PreUpdate)
            .add_schedule(Update)
            .add_schedule(PostUpdate)
            .add_schedule(CleanupHandles)
            .add_schedule(ApplyCommands)
            .add_schedule(Last);
    }
}
