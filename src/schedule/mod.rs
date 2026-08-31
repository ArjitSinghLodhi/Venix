use crate::{
    app::{App, plugin::Plugin},
    schedule::schedules_list::{ApplyCommands, CleanupHandles, First, Last, Update},
};

pub mod schedule;
pub mod schedules_list;

pub struct DefaultSchedulesPlugin;

impl Plugin for DefaultSchedulesPlugin {
    fn build(&mut self, app: &mut App) {
        app.add_schedule(First)
            .add_schedule(Update)
            .add_schedule(CleanupHandles)
            .add_schedule(ApplyCommands)
            .add_schedule(Last);
    }
}
