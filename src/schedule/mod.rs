use crate::{
    app::{App, Plugin},
    schedule::schedules_list::{
        ApplyCommands, CleanupHandles, First, Last, PostUpdate, PreUpdate, Update,
    },
};

pub mod schedule;
pub mod schedules_list;

/// The plugin to register the default schedules list: (`First`, `PreUpdate`, `Update`, `PostUpdate`, `CleanupHandles`, `ApplyCommands`, `Last`).
///
/// Out of all these, the `CleanupHandles` and `ApplyCommands` schedules are special:
///
/// * **`CleanupHandles`**: Queue commands are applied right before your systems run, as well as immediately after all your systems have run.
/// * **`ApplyCommands`**: Your systems run, then queue commands are applied, and finally despawns are applied.
///
/// This is a deliberate mechanism built to help you use the [`Commands::despawn_iter()`] and [`Commands::will_despawn()`]
/// functions to obey Venix's strict rule: *All cloned handles referencing an entity must be dropped before the entity's despawn command is applied.*
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
