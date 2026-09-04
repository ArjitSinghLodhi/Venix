use crate::{
    schedule::schedule::{IntoScheduleId, ScheduleLabel, SchedulePlace},
    system::validation::System,
    world::storage::World,
};

pub struct Startup;

impl ScheduleLabel for Startup {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::Before(First::id())
    }

    fn runner_fn() -> fn(&mut dyn ScheduleLabel, &mut World, &mut [Box<dyn System>])
    where
        Self: Sized,
    {
        |_, world, systems| {
            for system in systems.iter_mut() {
                system.run(world);
                world.apply_commands();
            }
        }
    }
}

pub struct First;

impl ScheduleLabel for First {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::After(Startup::id())
    }
}

pub struct PreUpdate;

impl ScheduleLabel for PreUpdate {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::After(First::id())
    }
}

pub struct Update;

impl ScheduleLabel for Update {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::After(PreUpdate::id())
    }
}

pub struct PostUpdate;

impl ScheduleLabel for PostUpdate {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::After(Update::id())
    }
}

/// A special schedule where queue commands are applied immediately before
/// and after the systems registered in this schedule run. Despawn commands are not applied yet.
///
/// See [`DefaultSchedulesPlugin`] for more information on the execution order.
///
/// [`DefaultSchedulesPlugin`]: crate::schedule::DefaultSchedulesPlugin
pub struct CleanupHandles;

impl ScheduleLabel for CleanupHandles {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::After(Update::id())
    }

    fn runner_fn() -> fn(&mut dyn ScheduleLabel, &mut World, &mut [Box<dyn System>])
    where
        Self: Sized,
    {
        |_, world, systems| {
            world.apply_queue_commands();
            for system in systems.iter_mut() {
                system.run(world);
            }
            world.apply_queue_commands();
        }
    }
}

/// A special schedule where systems registered in this schedule run,
/// followed by the execution of queue commands, and finally, despawn commands are processed.
///
/// See [`DefaultSchedulesPlugin`] for more information on the execution order.
///
/// [`DefaultSchedulesPlugin`]: crate::schedule::DefaultSchedulesPlugin
pub struct ApplyCommands;

impl ScheduleLabel for ApplyCommands {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::After(CleanupHandles::id())
    }
    fn runner_fn() -> fn(&mut dyn ScheduleLabel, &mut World, &mut [Box<dyn System>])
    where
        Self: Sized,
    {
        |_, world, systems| {
            for system in systems.iter_mut() {
                system.run(world);
            }
            world.apply_queue_commands();
            world.apply_despawns();
        }
    }
}

pub struct Last;

impl ScheduleLabel for Last {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::After(ApplyCommands::id())
    }
}
