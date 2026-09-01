use crate::{
    app::{App, Plugin},
    schedule::schedules_list::{
        ApplyCommands, CleanupHandles, First, Last, PostUpdate, PreUpdate, Update,
    },
};

pub mod schedule;
pub mod schedules_list;

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
