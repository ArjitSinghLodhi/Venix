use std::{
    cell::UnsafeCell,
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

pub struct ParallelEventWriter<T: 'static + Send> {
    master_buffer: *mut EventBuffer<T>,
    generation: u8,
    system_id: u32,
}

unsafe impl<T: 'static + Send> Send for ParallelEventWriter<T> {}
unsafe impl<T: 'static + Send> Sync for ParallelEventWriter<T> {}

impl<T: 'static + Send> ParallelEventWriter<T> {
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
                    local_data: slot.data.get(),
                    origin: EventWriterOrigin::ThreadLocal(std::mem::transmute(&slot.is_busy)),
                    master_buffer: self.master_buffer,
                    generation: self.generation,
                    system_id: self.system_id,
                    _marker: std::marker::PhantomData,
                }
            } else {
                let heap_box = Box::new(EventQueue::new());
                let heap_ptr = Box::into_raw(heap_box);

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

impl<T: 'static + Send + Sync> SystemParam for ParallelEventWriter<T> {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, system_data: &mut FunctionData) -> Self {
        let buffer_ptr = world.get_resource_mut::<EventBuffer<T>>() as *mut EventBuffer<T>;
        let generation_data = system_data.get_data::<FunctionGenerationData>().unwrap();

        Self {
            master_buffer: buffer_ptr,
            generation: GenerationRing::current(),
            system_id: generation_data.system_id,
        }
    }
}
