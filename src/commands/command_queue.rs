use std::alloc::Layout;
use std::mem::{self, MaybeUninit};
use std::ptr;

use crate::world::storage::World;

pub trait WorldCommand: Send + Sync + 'static {
    fn apply(self, world: &mut World);
}

struct CommandMeta {
    consume_and_advance: unsafe fn(payload_ptr: *mut u8, world: Option<&mut World>),
    payload_offset: usize,
    block_size: usize,
}

#[derive(Default)]
pub struct CommandQueue {
    bytes: Vec<MaybeUninit<u8>>,
}

impl CommandQueue {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn push<C: WorldCommand>(&mut self, command: C) {
        let meta_layout = Layout::new::<CommandMeta>();
        let payload_layout = Layout::new::<C>();

        let (combined_layout, payload_offset) = meta_layout
            .extend(payload_layout)
            .expect("Command layout overflowed memory bounds");

        let final_layout = combined_layout.pad_to_align();
        let block_size = final_layout.size();
        let old_len = self.bytes.len();

        self.bytes.reserve(block_size);

        let meta = CommandMeta {
            payload_offset,
            block_size,
            consume_and_advance: |payload_ptr, world| {
                let command: C = unsafe { ptr::read(payload_ptr.cast()) };

                match world {
                    Some(w) => command.apply(w),
                    None => mem::drop(command),
                }
            },
        };

        unsafe {
            let base_ptr = self.bytes.as_mut_ptr().add(old_len).cast::<u8>();

            ptr::write(base_ptr.cast::<CommandMeta>(), meta);
            let payload_target = base_ptr.add(payload_offset).cast::<C>();
            ptr::write(payload_target, command);

            self.bytes.set_len(old_len + block_size);
        }
    }

    /*pub fn push_fn<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World) + Send + Sync + 'static,
    {
        struct FuncCommand<F>(F);
        impl<F: FnOnce(&mut World) + Send + Sync + 'static> WorldCommand for FuncCommand<F> {
            fn apply(self, world: &mut World) {
                (self.0)(world);
            }
        }
        self.push(FuncCommand(f));
    }*/

    pub fn apply(&mut self, world: &mut World) {
        let mut local_cursor = 0;
        let total_bytes = self.bytes.len();

        while local_cursor < total_bytes {
            unsafe {
                let base_ptr = self.bytes.as_mut_ptr().add(local_cursor).cast::<u8>();

                let meta: CommandMeta = ptr::read(base_ptr.cast());

                let payload_ptr = base_ptr.add(meta.payload_offset);

                (meta.consume_and_advance)(payload_ptr, Some(world));

                local_cursor += meta.block_size;
            }
        }

        self.bytes.clear();
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        let mut local_cursor = 0;
        let total_bytes = self.bytes.len();
        while local_cursor < total_bytes {
            unsafe {
                let base_ptr = self.bytes.as_mut_ptr().add(local_cursor).cast::<u8>();
                let meta: CommandMeta = ptr::read(base_ptr.cast());
                let payload_ptr = base_ptr.add(meta.payload_offset);

                (meta.consume_and_advance)(payload_ptr, None);

                local_cursor += meta.block_size;
            }
        }
    }
}
