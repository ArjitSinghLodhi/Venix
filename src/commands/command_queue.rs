use std::alloc::Layout;
use std::mem::{self, MaybeUninit};
use std::ptr;

use crate::world::storage::World;

pub(crate) trait WorldCommand: 'static + Send {
    fn apply(self, world: &mut World);
}

struct CommandMeta {
    consume_and_advance: unsafe fn(payload_ptr: *mut u8, world: Option<&mut World>),
    move_to_dest: unsafe fn(payload_ptr: *mut u8, dst: &mut CommandQueue),
    payload_offset: usize,
    block_size: usize,
}

#[derive(Default)]
pub(crate) struct CommandQueue {
    u64_chunks: Vec<MaybeUninit<u64>>,
}

impl CommandQueue {
    pub(crate) fn new() -> Self {
        Self {
            u64_chunks: Vec::new(),
        }
    }

    pub(crate) fn push<C: WorldCommand>(&mut self, command: C) {
        let meta_layout = Layout::new::<CommandMeta>();
        let payload_layout = Layout::new::<C>();

        let (combined_layout, payload_offset) = meta_layout
            .extend(payload_layout)
            .expect("Command layout overflowed memory bounds");

        let final_layout = combined_layout.pad_to_align();
        let block_size = final_layout.size();

        let old_byte_len = self.u64_chunks.len() * mem::size_of::<u64>();
        let meta_align = mem::align_of::<CommandMeta>();

        let aligned_old_bytes = (old_byte_len + meta_align - 1) & !(meta_align - 1);
        let padding_gap = aligned_old_bytes - old_byte_len;

        let total_new_bytes = aligned_old_bytes + block_size;
        let target_u64_count = total_new_bytes.div_ceil(mem::size_of::<u64>());

        let old_u64_count = self.u64_chunks.len();
        self.u64_chunks.reserve(target_u64_count - old_u64_count);

        unsafe {
            let base_alloc_ptr = self.u64_chunks.as_mut_ptr().cast::<u8>().add(old_byte_len);
            ptr::write_bytes(base_alloc_ptr, 0, padding_gap);
            let base_ptr = self
                .u64_chunks
                .as_mut_ptr()
                .cast::<u8>()
                .add(aligned_old_bytes);
            let meta = CommandMeta {
                payload_offset,
                block_size,
                consume_and_advance: |payload_ptr, world| {
                    let command: C = ptr::read_unaligned(payload_ptr.cast());

                    match world {
                        Some(w) => command.apply(w),
                        None => mem::drop(command),
                    }
                },
                move_to_dest: |payload_ptr, dest| {
                    let command: C = ptr::read_unaligned(payload_ptr.cast());
                    dest.push(command);
                },
            };
            ptr::write(base_ptr.cast::<CommandMeta>(), meta);
            let payload_target = base_ptr.add(payload_offset).cast::<C>();
            ptr::write_unaligned(payload_target, command);
            self.u64_chunks.set_len(target_u64_count);
        }
    }

    pub(crate) fn push_fn<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World) + Send + 'static,
    {
        self.push(FunctionCommand { func: f });
    }

    pub(crate) fn apply(&mut self, world: &mut World) {
        let total_bytes = self.u64_chunks.len() * mem::size_of::<u64>();
        if total_bytes == 0 {
            return;
        }

        let mut local_cursor = 0;
        let base_mut_ptr = self.u64_chunks.as_mut_ptr().cast::<u8>();

        while local_cursor < total_bytes {
            unsafe {
                let meta_align = mem::align_of::<CommandMeta>();
                local_cursor = (local_cursor + meta_align - 1) & !(meta_align - 1);

                if local_cursor >= total_bytes {
                    break;
                }

                let base_ptr = base_mut_ptr.add(local_cursor);
                let meta: CommandMeta = ptr::read(base_ptr.cast());
                let payload_ptr = base_ptr.add(meta.payload_offset);

                (meta.consume_and_advance)(payload_ptr, Some(world));

                local_cursor += meta.block_size;
            }
        }

        self.u64_chunks.clear();
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        let total_bytes = self.u64_chunks.len() * mem::size_of::<u64>();
        if total_bytes == 0 {
            return;
        }

        let mut local_cursor = 0;
        let base_mut_ptr = self.u64_chunks.as_mut_ptr().cast::<u8>();

        while local_cursor < total_bytes {
            unsafe {
                let meta_align = mem::align_of::<CommandMeta>();
                local_cursor = (local_cursor + meta_align - 1) & !(meta_align - 1);

                if local_cursor >= total_bytes {
                    break;
                }

                let base_ptr = base_mut_ptr.add(local_cursor);
                let meta: CommandMeta = ptr::read(base_ptr.cast());
                let payload_ptr = base_ptr.add(meta.payload_offset);

                (meta.consume_and_advance)(payload_ptr, None);

                local_cursor += meta.block_size;
            }
        }
    }
}

impl CommandQueue {
    pub(crate) fn is_empty(&self) -> bool {
        self.u64_chunks.is_empty()
    }

    pub(crate) fn clear_bytes(&mut self) {
        self.u64_chunks.clear();
    }

    pub(crate) fn merge(&mut self, other: &mut CommandQueue) {
        if other.is_empty() {
            return;
        }

        let total_bytes = other.u64_chunks.len() * mem::size_of::<u64>();
        let mut local_cursor = 0;
        let base_mut_ptr = other.u64_chunks.as_mut_ptr().cast::<u8>();

        while local_cursor < total_bytes {
            unsafe {
                let meta_align = mem::align_of::<CommandMeta>();
                local_cursor = (local_cursor + meta_align - 1) & !(meta_align - 1);

                if local_cursor >= total_bytes {
                    break;
                }
                let base_ptr = base_mut_ptr.add(local_cursor);
                let meta: CommandMeta = ptr::read(base_ptr.cast());
                let payload_ptr = base_ptr.add(meta.payload_offset);
                (meta.move_to_dest)(payload_ptr, self);
                local_cursor += meta.block_size;
            }
        }
        other.clear_bytes();
    }
}

struct FunctionCommand<F>
where
    F: FnOnce(&mut World),
{
    func: F,
}

impl<T: Send + 'static + for<'a> FnOnce(&'a mut World)> WorldCommand for FunctionCommand<T> {
    fn apply(self, world: &mut World) {
        (self.func)(world)
    }
}
