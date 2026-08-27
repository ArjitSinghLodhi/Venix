use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    events::{
        EventBuffer, EventWriter,
        events::{EventLocalBufferSlot, EventQueue, EventWriterOrigin},
    },
    extensions::{FunctionData, ParamAccess, SystemParam, World},
    system::validation::FunctionGenerationData,
    world::storage::GenerationRing,
};

pub struct ParallelEventWriter<'w, T: 'static + Send> {
    master_buffer: *const EventBuffer<T>,
    generation: u8,
    system_id: u32,
    _marker: PhantomData<&'w ()>,
}

unsafe impl<'w, T: 'static + Send> Send for ParallelEventWriter<'w, T> {}
unsafe impl<'w, T: 'static + Send> Sync for ParallelEventWriter<'w, T> {}

impl<'w, T: 'static + Send> ParallelEventWriter<'w, T> {
    pub fn scope<F, R>(&self, f: F) -> R
    where
        F: for<'b> FnOnce(EventWriter<'b, T>) -> R,
    {
        unsafe {
            let slot = (*self.master_buffer)
                .local_buffers
                .get_or(|| EventLocalBufferSlot {
                    is_busy: AtomicBool::new(false),
                    data: UnsafeCell::new(EventQueue::new()),
                });

            let writer = if slot
                .is_busy
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                EventWriter {
                    local_data: NonNull::new_unchecked(slot.data.get()),
                    origin: EventWriterOrigin::ThreadLocal(&slot.is_busy as *const AtomicBool),
                    master_buffer: self.master_buffer,
                    generation: self.generation,
                    system_id: self.system_id,
                    _marker: std::marker::PhantomData,
                }
            } else {
                let heap_box = Box::new(EventQueue::new());
                let heap_ptr = NonNull::new_unchecked(Box::into_raw(heap_box));

                EventWriter {
                    local_data: heap_ptr,
                    origin: EventWriterOrigin::HeapFallback(heap_ptr),
                    master_buffer: self.master_buffer,
                    generation: self.generation,
                    system_id: self.system_id,
                    _marker: std::marker::PhantomData,
                }
            };
            f(writer)
        }
    }
}

impl<'w, T: 'static + Send> SystemParam for ParallelEventWriter<'w, T> {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, system_data: &mut FunctionData) -> Self {
        let buffer_ptr = world.get_resource::<EventBuffer<T>>() as *const EventBuffer<T>;
        let generation_data = system_data.get_data::<FunctionGenerationData>().unwrap();

        Self {
            master_buffer: buffer_ptr,
            generation: GenerationRing::current(),
            system_id: generation_data.system_id,
            _marker: PhantomData,
        }
    }
}
