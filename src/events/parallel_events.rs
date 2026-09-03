use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    events::{EventBuffer, EventReader, EventWriter, events::EventQueue},
    extensions::{FunctionData, ParamAccess, SystemParam, World},
};

/// A thread-safe, thread-clonable handle that acts as a detached remote input for dispatching events.
///
/// `ParallelEventWriter` can be passed to external or background worker threads, allowing them to
/// concurrently dispatch events outside the main execution path. It can be obtained directly from
/// the application layer via [`.get_par_event_writer()`].
///
/// # Deferred Actions & Invariants
///
/// Any events queued through this handle remain subject to the engine's strict double-buffered,
/// frame-locked 3-frame lifecycle. Events pushed here are buffered in the current frame and
/// become globally readable in the next frame.
///
/// [`.get_par_event_writer()`]: crate::app::App::get_par_event_writer
#[derive(Clone)]
pub struct ParallelEventWriter<T: 'static + Send + Sync> {
    write_buffer: Arc<RwLock<EventQueue<T>>>,
}

impl<T: 'static + Send + Sync> ParallelEventWriter<T> {
    /// Creates a temporary [`EventWriter`] context bound to the current scope block.
    ///
    /// This allows external or parallel tasks to safely dispatch events using the engine's
    /// standard event pipeline syntax via an inner closure.
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

/// A thread-safe, thread-clonable handle that acts as a detached remote reader for inspecting events.
///
/// `ParallelEventReader` can be passed to external or background worker threads, allowing them to
/// concurrently inspect dispatched events outside the main execution path. It can be obtained directly
/// from the application layer via [`.get_par_event_reader()`].
///
/// # Critical Synchronization Warning
///
/// Because events follow the engine's global 3-frame lifecycle, **they are unconditionally cleared on frame 3
/// regardless of system execution.**
///
/// When using this handle on external threads, you must implement manual synchronization with the main ECS
/// loop. If the main application ticks too fast or advances frames before your parallel thread processes
/// the current window, the engine will clear the underlying buffer, causing the handle to miss data entirely.
///
/// [`.get_par_event_reader()`]: crate::app::App::get_par_event_reader
#[derive(Clone)]
pub struct ParallelEventReader<T: 'static + Send + Sync> {
    read_buffer: Arc<RwLock<EventQueue<T>>>,
}

impl<T: 'static + Send + Sync> ParallelEventReader<T> {
    /// Creates a temporary [`EventReader`] wrapper context bound to the current scope block.
    ///
    /// This allows external or parallel tasks to safely iterate over events using the engine's
    /// standard event pipeline syntax via an inner closure.
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
