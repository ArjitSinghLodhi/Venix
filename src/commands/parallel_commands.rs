use crate::commands::commands::{
    CommandBuffer, Commands, CommandsBufferData, CommandsOrigin, LocalSlot,
};
use crate::extensions::{ParamAccess, SystemParam};
use crate::system::validation::FunctionData;
use crate::world::storage::World;
use std::cell::UnsafeCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct ParallelCommands {
    pub(crate) master_buffer_ptr: *mut CommandBuffer,
}

unsafe impl Send for ParallelCommands {}
unsafe impl Sync for ParallelCommands {}

impl SystemParam for ParallelCommands {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, _data: &mut FunctionData) -> Self {
        Self {
            master_buffer_ptr: Arc::as_ptr(&world.commands) as *mut CommandBuffer,
        }
    }
}

impl ParallelCommands {
    pub fn scope<F, R>(&self, f: F) -> R
    where
        F: for<'b> FnOnce(Commands<'b>) -> R,
    {
        unsafe {
            let slot = (*self.master_buffer_ptr).local_data.get_or(|| LocalSlot {
                is_busy: AtomicBool::new(false),
                data: UnsafeCell::new(CommandsBufferData::new()),
            });

            if slot
                .is_busy
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                let commands = Commands {
                    local_data: slot.data.get(),
                    origin: CommandsOrigin::ThreadLocal(&slot.is_busy as *const AtomicBool),
                    master_buffer: self.master_buffer_ptr,
                    _marker: std::marker::PhantomData,
                };
                f(commands)
            } else {
                let heap_box = Box::new(CommandsBufferData::new());
                let heap_ptr = Box::into_raw(heap_box);

                let commands = Commands {
                    local_data: heap_ptr,
                    origin: CommandsOrigin::HeapFallback(heap_ptr),
                    master_buffer: self.master_buffer_ptr,
                    _marker: std::marker::PhantomData,
                };
                f(commands)
            }
        }
    }
}
