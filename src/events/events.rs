use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    marker::PhantomData,
    sync::{
        RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use thread_local::ThreadLocal;

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

pub(crate) fn register_event<T: 'static + Send>() {
    let mut tracked = TRACKED_EVENTS.write().unwrap();
    if let None = tracked
        .iter()
        .find(|meta| meta.comp_id == TypeId::of::<T>())
    {
        tracked.push(TrackedEventsMeta {
            comp_id: TypeId::of::<T>(),
            event_id: TypeId::of::<EventBuffer<T>>(),
            clear_events: |raw_unsafecell| {
                let cell = raw_unsafecell.get_mut();
                let event_queue = cell
                    .downcast_mut::<EventBuffer<T>>()
                    .expect("Registered event queue was not found when clearing data");

                let current_generation = GenerationRing::current();
                let stale_generation = GenerationRing::stale_threshold(current_generation);
                event_queue
                    .master_queue
                    .write()
                    .unwrap()
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

pub(crate) enum EventWriterOrigin<T: 'static + Send> {
    ThreadLocal(*const AtomicBool),
    HeapFallback(*mut EventQueue<T>),
}

pub(crate) struct EventLocalBufferSlot<T: 'static + Send> {
    pub(crate) is_busy: AtomicBool,
    pub(crate) data: UnsafeCell<EventQueue<T>>,
}

pub(crate) struct EventBuffer<T: 'static + Send> {
    pub(crate) master_queue: RwLock<EventQueue<T>>,
    pub(crate) local_buffers: ThreadLocal<EventLocalBufferSlot<T>>,
}

impl<T: 'static + Send> EventBuffer<T> {
    pub(crate) fn new() -> Self {
        EventBuffer {
            master_queue: RwLock::new(EventQueue::new()),
            local_buffers: ThreadLocal::new(),
        }
    }
}

impl<T: 'static> EventQueue<T> {
    pub(crate) fn new() -> Self {
        Self { queue: Vec::new() }
    }
}

pub struct EventWriter<'a, T: 'static + Send> {
    pub(crate) local_data: *mut EventQueue<T>,
    pub(crate) origin: EventWriterOrigin<T>,
    pub(crate) master_buffer: *const EventBuffer<T>,
    pub(crate) generation: u8,
    pub(crate) system_id: u32,
    pub(crate) _marker: PhantomData<&'a ()>,
}

impl<'a, T: 'static + Send> EventWriter<'a, T> {
    #[inline]
    pub fn send(&mut self, event: T) {
        unsafe {
            (*self.local_data)
                .queue
                .push(Event::new(event, self.generation, self.system_id));
        }
    }
}
impl<'a, T: 'static + Send> SystemParam for EventWriter<'a, T> {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, system_data: &mut FunctionData) -> Self {
        unsafe {
            let buffer_ptr = world.get_resource_mut::<EventBuffer<T>>() as *mut EventBuffer<T>;
            let buffer_ref: &'a EventBuffer<T> = &*buffer_ptr;

            let slot = buffer_ref.local_buffers.get_or(|| EventLocalBufferSlot {
                is_busy: AtomicBool::new(false),
                data: UnsafeCell::new(EventQueue::new()),
            });

            let generation_data = system_data.get_data::<FunctionGenerationData>().unwrap();
            let current_gen = GenerationRing::current();
            let sys_id = generation_data.system_id;

            if slot
                .is_busy
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                Self {
                    local_data: slot.data.get(),
                    origin: EventWriterOrigin::ThreadLocal(&slot.is_busy as *const AtomicBool),
                    master_buffer: buffer_ptr,
                    generation: current_gen,
                    system_id: sys_id,
                    _marker: PhantomData,
                }
            } else {
                let heap_box = Box::new(EventQueue::new());
                let heap_ptr = Box::into_raw(heap_box);
                Self {
                    local_data: heap_ptr,
                    origin: EventWriterOrigin::HeapFallback(heap_ptr),
                    master_buffer: buffer_ptr,
                    generation: current_gen,
                    system_id: sys_id,
                    _marker: PhantomData,
                }
            }
        }
    }
}

impl<'a, T: 'static + Send> Drop for EventWriter<'a, T> {
    fn drop(&mut self) {
        unsafe {
            let local_vec = &mut *self.local_data;
            if !local_vec.queue.is_empty() {
                let mut master_queue = (*self.master_buffer).master_queue.write().unwrap();
                master_queue.queue.extend(local_vec.queue.drain(..));
            }

            match self.origin {
                EventWriterOrigin::ThreadLocal(is_busy_flag) => {
                    (*is_busy_flag).store(false, Ordering::Relaxed);
                }
                EventWriterOrigin::HeapFallback(heap_ptr) => {
                    let _cleanup = Box::from_raw(heap_ptr);
                }
            }
        }
    }
}

pub struct EventIterator<'a, T: 'static> {
    _guard: std::sync::RwLockReadGuard<'a, EventQueue<T>>,
    index: usize,
    current_gen: u8,
    system_last_gen: u8,
    previous_gen: u8,
    reading_sys_id: u32,
}

impl<'a, T: 'static> Iterator for EventIterator<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self._guard.queue.len() {
            let event_ptr = &self._guard.queue[self.index] as *const Event<T>;
            self.index += 1;

            unsafe {
                let event = &*event_ptr;
                if should_be_read(
                    event.generation,
                    event.author_id,
                    self.current_gen,
                    self.system_last_gen,
                    self.previous_gen,
                    self.reading_sys_id,
                ) {
                    return Some(&event.event);
                }
            }
        }
        None
    }
}

pub struct EventReader<'w, T: 'static + Send> {
    system_last_gen: u8,
    previous_gen: u8,
    current_gen: u8,
    reading_system_id: u32,
    queue: *const EventBuffer<T>,
    _marker: PhantomData<(&'w (), T)>,
}

impl<'w, T: 'static + Send> EventReader<'w, T> {
    pub fn read_scope<F, R>(&self, f: F) -> R
    where
        F: for<'a> FnOnce(EventIterator<'a, T>) -> R,
    {
        let current_gen = self.current_gen;
        let system_last_gen = self.system_last_gen;
        let previous_gen = self.previous_gen;
        let reading_sys_id = self.reading_system_id;

        unsafe {
            let guard = (&*self.queue).master_queue.read().unwrap();

            let iter = EventIterator {
                _guard: guard,
                index: 0,
                current_gen,
                system_last_gen,
                previous_gen,
                reading_sys_id,
            };

            f(iter)
        }
    }
}

impl<'w, T: 'static + Send> SystemParam for EventReader<'w, T> {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, system_data: &mut FunctionData) -> Self {
        let event_queue_ptr = world.get_resource::<EventBuffer<T>>() as *const EventBuffer<T>;

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
            queue: event_queue_ptr,
            _marker: PhantomData,
        }
    }
}

unsafe impl<'w, T: 'static + Send> Send for EventReader<'w, T> {}
unsafe impl<'w, T: 'static + Send> Sync for EventReader<'w, T> {}

pub fn should_be_read(
    event_generation: u8,
    author_system_id: u32,
    current_generation: u8,
    system_last_generation: u8,
    previous_generation: u8,
    reading_system_id: u32,
) -> bool {
    if event_generation == 0 {
        return false;
    }
    let is_current_generation = event_generation == current_generation;
    let is_previous_generation = event_generation == previous_generation;

    let reading_greater_than_author = reading_system_id > author_system_id;
    let reading_less_than_author = reading_system_id < author_system_id;

    let current_generation_result =
        (system_last_generation != current_generation) & reading_greater_than_author;

    let two_generations_ago = GenerationRing::stale_threshold(current_generation);
    let previous_generation_result = if system_last_generation == previous_generation {
        reading_less_than_author
    } else {
        system_last_generation == two_generations_ago
    };

    (is_current_generation & current_generation_result)
        | (is_previous_generation & previous_generation_result)
}
