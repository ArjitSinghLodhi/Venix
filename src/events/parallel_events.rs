use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    events::{EventBuffer, EventReader, EventWriter, events::EventQueue},
    extensions::{FunctionData, ParamAccess, SystemParam, World},
};

#[derive(Clone)]
pub struct ParallelEventWriter<T: 'static + Send + Sync> {
    write_buffer: Arc<RwLock<EventQueue<T>>>,
}

impl<T: 'static + Send + Sync> ParallelEventWriter<T> {
    pub fn scope<F, R>(&self, f: F) -> R
    where
        F: for<'b> FnOnce(EventWriter<'b, T>) -> R,
    {
        let writer = EventWriter {
            write_buffer: self.write_buffer.read(),
        };
        f(writer)
    }
}

impl<T: 'static + Send + Sync> SystemParam for ParallelEventWriter<T> {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, _system_data: &mut FunctionData) -> Self {
        let buffer_ptr = world.get_resource::<EventBuffer<T>>() as *const EventBuffer<T>;
        let queue = unsafe { (*buffer_ptr).writer_queue.clone() };
        Self {
            write_buffer: queue,
        }
    }
}

unsafe impl<T: 'static + Send + Sync> Send for ParallelEventWriter<T> {}
unsafe impl<T: 'static + Send + Sync> Sync for ParallelEventWriter<T> {}

#[derive(Clone)]
pub struct ParallelEventReader<T: 'static + Send + Sync> {
    read_buffer: Arc<RwLock<EventQueue<T>>>,
}

impl<T: 'static + Send + Sync> ParallelEventReader<T> {
    pub fn scope<F, R>(&self, f: F) -> R
    where
        F: for<'b> FnOnce(EventReader<'b, T>) -> R,
    {
        let reader = EventReader {
            read_buffer: self.read_buffer.read(),
        };
        f(reader)
    }
}

impl<T: 'static + Send + Sync> SystemParam for ParallelEventReader<T> {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, _system_data: &mut FunctionData) -> Self {
        let buffer_ptr = world.get_resource::<EventBuffer<T>>() as *const EventBuffer<T>;
        let queue = unsafe { (*buffer_ptr).read_queue.clone() };
        Self { read_buffer: queue }
    }
}

unsafe impl<T: 'static + Send + Sync> Send for ParallelEventReader<T> {}
unsafe impl<T: 'static + Send + Sync> Sync for ParallelEventReader<T> {}
