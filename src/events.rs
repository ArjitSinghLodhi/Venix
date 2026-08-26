use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    marker::PhantomData,
    sync::RwLock,
};

use crate::{
    extensions::{FunctionData, ParamAccess, SystemParam, World},
    system::validation::FunctionGenerationData,
    world::storage::GenerationRing,
};

pub(crate) struct TrackedEventsMeta {
    pub(crate) comp_id: TypeId,
    pub(crate) event_id: TypeId,
    pub(crate) clear_events: fn(&mut UnsafeCell<Box<dyn Any>>),
}

pub(crate) static TRACKED_EVENTS: RwLock<Vec<TrackedEventsMeta>> = RwLock::new(Vec::new());

pub(crate) fn register_event<T: 'static>() {
    let mut tracked = TRACKED_EVENTS.write().unwrap();
    if let None = tracked
        .iter()
        .find(|meta| meta.comp_id == TypeId::of::<T>())
    {
        tracked.push(TrackedEventsMeta {
            comp_id: TypeId::of::<T>(),
            event_id: TypeId::of::<EventQueue<T>>(),
            clear_events: |raw_unsafecell| {
                let cell = raw_unsafecell.get_mut();
                let event_queue = cell
                    .downcast_mut::<EventQueue<T>>()
                    .expect("Registered event queue was not found when clearing data");

                let current_generation = GenerationRing::current();
                let stale_generation = GenerationRing::stale_threshold(current_generation);
                event_queue
                    .queue
                    .retain(|event| event.generation != stale_generation);
            },
        });
    }
}

struct Event<T: 'static> {
    event: T,
    generation: u8,
    author_id: u32,
}

impl<T: 'static> Event<T> {
    pub(crate) fn new(event: T, generation: u8, author_id: u32) -> Self {
        Self {
            event,
            generation,
            author_id,
        }
    }
}

pub(crate) struct EventQueue<T: 'static> {
    queue: Vec<Event<T>>,
}

impl<T: 'static> EventQueue<T> {
    pub(crate) fn new() -> Self {
        Self { queue: Vec::new() }
    }
}

pub struct EventWriter<T: 'static> {
    generation: u8,
    system_id: u32,
    queue: *mut EventQueue<T>,
}

impl<T: 'static> EventWriter<T> {
    pub fn send(&mut self, event: T) {
        unsafe {
            (*self.queue)
                .queue
                .push(Event::new(event, self.generation, self.system_id));
        }
    }
}

impl<T: 'static> SystemParam for EventWriter<T> {
    fn get_access() -> crate::extensions::ParamAccess {
        let mut access = ParamAccess::default();
        access.res_writes.push(TypeId::of::<EventQueue<T>>());
        access
    }

    fn extract(world: &mut World, system_data: &mut FunctionData) -> Self {
        let event_queue = world.get_resource_mut::<EventQueue<T>>() as *mut EventQueue<T>;
        let generation_data = system_data.get_data::<FunctionGenerationData>().unwrap();

        Self {
            generation: GenerationRing::current(),
            system_id: generation_data.system_id,
            queue: event_queue,
        }
    }
}

pub struct EventReader<T: 'static> {
    pub(crate) system_last_gen: u8,
    pub(crate) previous_gen: u8,
    pub(crate) current_gen: u8,
    pub(crate) reading_system_id: u32,
    pub(crate) queue: *const EventQueue<T>,
    pub(crate) _marker: PhantomData<T>,
}

unsafe impl<T: 'static> Send for EventReader<T> {}
unsafe impl<T: 'static> Sync for EventReader<T> {}

impl<T: 'static> EventReader<T> {
    pub fn read(&self) -> impl Iterator<Item = &T> {
        let current_gen = self.current_gen;
        let system_last_gen = self.system_last_gen;
        let previous_gen = self.previous_gen;
        let reading_sys_id = self.reading_system_id;

        unsafe {
            self.queue
                .as_ref()
                .into_iter()
                .flat_map(|q| &q.queue)
                .filter(move |event| {
                    should_be_read(
                        event.generation,
                        event.author_id,
                        current_gen,
                        system_last_gen,
                        previous_gen,
                        reading_sys_id,
                    )
                })
                .map(|envelope| &envelope.event)
        }
    }
}

impl<T: 'static> SystemParam for EventReader<T> {
    fn get_access() -> crate::extensions::ParamAccess {
        let mut access = ParamAccess::default();
        access.res_reads.push(TypeId::of::<EventQueue<T>>());
        access
    }

    fn extract(world: &mut World, system_data: &mut FunctionData) -> Self {
        let event_queue = world.get_resource::<EventQueue<T>>() as *const EventQueue<T>;

        let current_gen = GenerationRing::current();
        let previous_gen = GenerationRing::previous(current_gen);
        let generation_data = system_data.get_data::<FunctionGenerationData>().unwrap();
        let system_last_gen = generation_data.last_run_generation;
        let reading_system_id = generation_data.system_id;

        Self {
            system_last_gen,
            previous_gen,
            current_gen,
            reading_system_id,
            queue: event_queue,
            _marker: PhantomData,
        }
    }
}

fn should_be_read(
    event_gen: u8,
    author_system_id: u32,
    current_generation: u8,
    system_last_generation: u8,
    previous_generation: u8,
    reading_system_id: u32,
) -> bool {
    if event_gen == 0 {
        return false;
    }
    if event_gen == current_generation {
        if system_last_generation == current_generation {
            return false;
        }
        return reading_system_id > author_system_id;
    }
    if event_gen == previous_generation {
        if system_last_generation == previous_generation {
            return reading_system_id < author_system_id;
        }

        let two_generations_ago = GenerationRing::stale_threshold(current_generation);
        return system_last_generation == two_generations_ago;
    }

    false
}
