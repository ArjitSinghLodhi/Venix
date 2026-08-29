use crate::world::storage::World;
use orx_concurrent_bag::ConcurrentBag;
use std::mem::{self, ManuallyDrop, MaybeUninit};
use std::ptr;

pub(crate) trait WorldCommand: 'static + Send {
    fn apply(self, world: &mut World);
}

struct CommandMeta {
    consume_and_advance:
        unsafe fn(payload_buf_ptr: *mut MaybeUninit<u64>, world: Option<&mut World>),
    u64_count: usize,
}

#[repr(C)]
struct ConfiguredCommand<C: WorldCommand> {
    meta: CommandMeta,
    payload: C,
}

pub(crate) struct CommandQueue {
    u64_chunks: ConcurrentBag<MaybeUninit<u64>>,
    scratchpad: Vec<MaybeUninit<u64>>,
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self {
            u64_chunks: ConcurrentBag::new(),
            scratchpad: Vec::with_capacity(64),
        }
    }
}

impl CommandQueue {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push<C: WorldCommand>(&self, command: C) {
        let block_size = mem::size_of::<ConfiguredCommand<C>>();
        let total_u64_count = block_size.div_ceil(mem::size_of::<u64>());

        let meta_u64_count = mem::size_of::<CommandMeta>().div_ceil(mem::size_of::<u64>());
        let payload_u64_count = total_u64_count - meta_u64_count;

        let meta = CommandMeta {
            u64_count: payload_u64_count,
            consume_and_advance: |payload_buf_ptr, world| unsafe {
                let command: C = ptr::read_unaligned(payload_buf_ptr.cast());
                match world {
                    Some(w) => command.apply(w),
                    None => mem::drop(command),
                }
            },
        };

        let stream_iter = StackCommandIterator {
            command: ManuallyDrop::new(ConfiguredCommand {
                meta,
                payload: command,
            }),
            total_u64_count,
            current_u64_idx: 0,
        };
        self.u64_chunks.extend(stream_iter);
    }

    pub(crate) fn push_fn<F>(&self, f: F)
    where
        F: FnOnce(&mut World) + Send + 'static,
    {
        self.push(FunctionCommand { func: f });
    }

    pub(crate) fn apply(&mut self, world: &mut World) {
        if self.u64_chunks.is_empty() {
            return;
        }

        let meta_u64_size = mem::size_of::<CommandMeta>().div_ceil(mem::size_of::<u64>());
        {
            let chunks_iter = &mut self.u64_chunks.iter_mut();
            while let Some(first_chunk_ref) = chunks_iter.next() {
                self.scratchpad.clear();
                self.scratchpad.push(*first_chunk_ref);
                for _ in 1..meta_u64_size {
                    if let Some(next_meta_chunk_ref) = chunks_iter.by_ref().next() {
                        self.scratchpad.push(*next_meta_chunk_ref);
                    }
                }

                let meta: CommandMeta = unsafe { ptr::read(self.scratchpad.as_ptr().cast()) };
                let total_needed = meta_u64_size + meta.u64_count;
                if self.scratchpad.capacity() < total_needed {
                    self.scratchpad
                        .reserve(total_needed - self.scratchpad.capacity());
                }

                for _ in 0..meta.u64_count {
                    if let Some(payload_chunk_ref) = chunks_iter.by_ref().next() {
                        self.scratchpad.push(*payload_chunk_ref);
                    }
                }

                unsafe {
                    let scratch_ptr = self.scratchpad.as_mut_ptr().cast::<u8>();
                    let payload_scratch_ptr = scratch_ptr
                        .add(mem::size_of::<CommandMeta>())
                        .cast::<MaybeUninit<u64>>();
                    (meta.consume_and_advance)(payload_scratch_ptr, Some(world));
                }
            }
        }
        self.u64_chunks.clear();
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        if self.u64_chunks.is_empty() {
            return;
        }
        let meta_u64_size = mem::size_of::<CommandMeta>().div_ceil(mem::size_of::<u64>());
        {
            let chunks_iter = &mut self.u64_chunks.iter_mut();
            while let Some(first_chunk_ref) = chunks_iter.next() {
                self.scratchpad.clear();
                self.scratchpad.push(*first_chunk_ref);
                for _ in 1..meta_u64_size {
                    if let Some(next_meta_chunk_ref) = chunks_iter.by_ref().next() {
                        self.scratchpad.push(*next_meta_chunk_ref);
                    }
                }

                let meta: CommandMeta = unsafe { ptr::read(self.scratchpad.as_ptr().cast()) };
                let total_needed = meta_u64_size + meta.u64_count;
                if self.scratchpad.capacity() < total_needed {
                    self.scratchpad
                        .reserve(total_needed - self.scratchpad.capacity());
                }

                for _ in 0..meta.u64_count {
                    if let Some(payload_chunk_ref) = chunks_iter.by_ref().next() {
                        self.scratchpad.push(*payload_chunk_ref);
                    }
                }

                unsafe {
                    let scratch_ptr = self.scratchpad.as_mut_ptr().cast::<u8>();
                    let payload_scratch_ptr = scratch_ptr
                        .add(mem::size_of::<CommandMeta>())
                        .cast::<MaybeUninit<u64>>();
                    (meta.consume_and_advance)(payload_scratch_ptr, None);
                }
            }
        }
        self.u64_chunks.clear();
    }
}
struct StackCommandIterator<C: WorldCommand> {
    command: ManuallyDrop<ConfiguredCommand<C>>,
    total_u64_count: usize,
    current_u64_idx: usize,
}

impl<C: WorldCommand> Iterator for StackCommandIterator<C> {
    type Item = MaybeUninit<u64>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_u64_idx >= self.total_u64_count {
            return None;
        }

        let byte_offset = self.current_u64_idx * mem::size_of::<u64>();
        let mut target_u64 = MaybeUninit::<u64>::uninit();

        unsafe {
            let source_ptr = (&*self.command as *const ConfiguredCommand<C>).cast::<u8>();
            let out_ptr = target_u64.as_mut_ptr().cast::<u8>();
            let struct_size = mem::size_of::<ConfiguredCommand<C>>();
            let bytes_left = struct_size.saturating_sub(byte_offset);

            if bytes_left >= mem::size_of::<u64>() {
                ptr::copy_nonoverlapping(
                    source_ptr.add(byte_offset),
                    out_ptr,
                    mem::size_of::<u64>(),
                );
            } else {
                ptr::write_bytes(out_ptr, 0, mem::size_of::<u64>());
                if bytes_left > 0 {
                    ptr::copy_nonoverlapping(source_ptr.add(byte_offset), out_ptr, bytes_left);
                }
            }
        }

        self.current_u64_idx += 1;
        Some(target_u64)
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total_u64_count - self.current_u64_idx;
        (remaining, Some(remaining))
    }
}

impl<C: WorldCommand> ExactSizeIterator for StackCommandIterator<C> {}

struct FunctionCommand<F>
where
    F: FnOnce(&mut World) + Send + 'static,
{
    func: F,
}

impl<F: FnOnce(&mut World) + Send + 'static> WorldCommand for FunctionCommand<F> {
    fn apply(self, world: &mut World) {
        (self.func)(world)
    }
}
