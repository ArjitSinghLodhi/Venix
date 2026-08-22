use crate::commands::commands::{CommandBuffer, Commands, CommandsBufferData};
use crate::extensions::{ParamAccess, SystemParam};
use crate::system::validation::FunctionData;
use crate::world::storage::World;
use std::cell::RefCell;

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
            let local_data_ptr = std::ptr::addr_of!((*self.master_buffer_ptr).local_data);
            let local_data = &*local_data_ptr;

            let data_cell = local_data.get_or(|| RefCell::new(CommandsBufferData::new()));

            match data_cell.try_borrow_mut() {
                Ok(data_borrow) => {
                    let commands = Commands {
                        local_data: data_borrow,
                        master_buffer_ptr: self.master_buffer_ptr,
                    };
                    f(commands);
                }
                _ => {
                    let fallback_data = CommandsBufferData::new();
                    let fallback_data_cell = RefCell::new(fallback_data);

                    let commands = Commands {
                        local_data: fallback_data_cell.borrow_mut(),
                        master_buffer_ptr: self.master_buffer_ptr,
                    };

                    f(commands);
                }
            }
        }
    }
}
