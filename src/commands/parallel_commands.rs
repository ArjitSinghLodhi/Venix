use crate::commands::command_queue::CommandQueue;
use crate::commands::commands::{CommandBuffer, Commands};
use crate::extensions::{ParamAccess, SystemParam};
use crate::system::validation::FunctionData;
use crate::world::storage::World;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashSet;

pub struct ParallelCommands {
    pub(crate) master_buffer_ptr: *mut CommandBuffer,
}

unsafe impl Send for ParallelCommands {}
unsafe impl Sync for ParallelCommands {}

impl SystemParam for ParallelCommands {
    fn get_access() -> ParamAccess {
        let mut access = ParamAccess::default();
        access
            .commands_accessed
            .push(TypeId::of::<ParallelCommands>());
        access
    }

    fn extract(world: &mut World, _data: &mut FunctionData) -> Self {
        Self {
            master_buffer_ptr: &mut world.commands as *mut CommandBuffer,
        }
    }
}

impl ParallelCommands {
    pub fn scope<F>(&self, f: F)
    where
        F: for<'b> FnOnce(Commands<'b>),
    {
        unsafe {
            let local_channels_ptr = std::ptr::addr_of!((*self.master_buffer_ptr).local_channels);
            let local_despawns_ptr = std::ptr::addr_of!((*self.master_buffer_ptr).local_despawns);
            let local_channels = &*local_channels_ptr;
            let local_despawns = &*local_despawns_ptr;

            let q_cell = local_channels.get_or(|| RefCell::new(CommandQueue::new()));
            let d_cell = local_despawns.get_or(|| RefCell::new(HashSet::new()));

            match (q_cell.try_borrow_mut(), d_cell.try_borrow_mut()) {
                (Ok(q_borrow), Ok(d_borrow)) => {
                    let commands = Commands {
                        local_queue: q_borrow,
                        local_despawns: d_borrow,
                        master_buffer_address: self.master_buffer_ptr as usize,
                    };
                    f(commands);
                }
                _ => {
                    let fallback_queue = CommandQueue::new();
                    let fallback_despawns = HashSet::new();
                    let q_cell = RefCell::new(fallback_queue);
                    let d_cell = RefCell::new(fallback_despawns);

                    let commands = Commands {
                        local_queue: q_cell.borrow_mut(),
                        local_despawns: d_cell.borrow_mut(),
                        master_buffer_address: self.master_buffer_ptr as usize,
                    };

                    f(commands);
                }
            }
        }
    }
}
