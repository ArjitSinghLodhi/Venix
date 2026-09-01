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

pub struct CleanupHandles;

impl ScheduleLabel for CleanupHandles {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::After(Update::id())
    }
}

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
            world.apply_commands();
        }
    }
}

pub struct Last;

impl ScheduleLabel for Last {
    fn get_place(&self) -> SchedulePlace {
        SchedulePlace::After(ApplyCommands::id())
    }
}
