use std::any::Any;

use crate::{system::validation::System, world::storage::World};

#[derive(Clone, Copy)]
pub struct ScheduleId {
    pub(crate) id: std::any::TypeId,
    pub(crate) name: &'static str,
}

pub trait IntoScheduleId<T: 'static>: ScheduleLabel {
    fn id() -> ScheduleId {
        ScheduleId {
            id: std::any::TypeId::of::<T>(),
            name: std::any::type_name::<T>(),
        }
    }
}

impl PartialEq for ScheduleId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: ScheduleLabel> IntoScheduleId<T> for T {}

pub enum SchedulePlace {
    Before(ScheduleId),
    After(ScheduleId),
}

pub trait ScheduleLabel
where
    Self: 'static,
{
    fn get_place(&self) -> SchedulePlace;

    fn as_any(&self) -> &dyn Any {
        unsafe {
            let thin_ptr = self as *const Self as *const ();
            &*(thin_ptr as *const (dyn Any + 'static))
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        unsafe {
            let thin_ptr = self as *mut Self as *mut ();
            &mut *(thin_ptr as *mut (dyn Any + 'static))
        }
    }

    fn runner_fn() -> fn(&mut dyn ScheduleLabel, &mut World, &Vec<Box<dyn System>>)
    where
        Self: Sized,
    {
        |_, world, systems| {
            for system in systems.iter() {
                system.run(world);
            }
        }
    }

    fn id_from_self(&self) -> ScheduleId {
        ScheduleId {
            id: self.type_id(),
            name: self.name(),
        }
    }

    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub(crate) struct Schedule {
    pub(crate) schedule: Box<dyn ScheduleLabel>,
    pub(crate) systems: Vec<Box<dyn System>>,
    pub(crate) runner_fn: fn(&mut dyn ScheduleLabel, &mut World, &Vec<Box<dyn System>>),
}

impl Schedule {
    pub fn new<T: ScheduleLabel + 'static>(schedule: T) -> Self {
        Self {
            schedule: Box::new(schedule),
            systems: Vec::new(),
            runner_fn: T::runner_fn(),
        }
    }

    pub fn run(&mut self, world: &mut World) {
        (self.runner_fn)(&mut *self.schedule, world, &self.systems);
    }
}
