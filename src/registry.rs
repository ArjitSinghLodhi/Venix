use crate::world::archetypes::ArchetypeId;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU32, Ordering};

pub(crate) struct RegistryCell {
    pub(crate) archetype_id: ArchetypeId,
    pub(crate) idx: u32,
    pub(crate) handle_count: AtomicU32,
}

pub(crate) struct UnsafeGlobalRegistry(UnsafeCell<Vec<RegistryCell>>);

unsafe impl Send for UnsafeGlobalRegistry {}
unsafe impl Sync for UnsafeGlobalRegistry {}

impl UnsafeGlobalRegistry {
    #[inline(always)]
    pub fn len(&self) -> usize {
        unsafe { (*self.0.get()).len() }
    }

    #[inline(always)]
    pub fn push(&self, cell: RegistryCell) {
        unsafe {
            (*self.0.get()).push(cell);
        }
    }

    #[inline(always)]
    pub unsafe fn get_ptr(&self, index: usize) -> *const RegistryCell {
        unsafe { (*self.0.get()).as_ptr().add(index) }
    }

    #[inline(always)]
    pub unsafe fn get_mut_ptr(&self, index: usize) -> *mut RegistryCell {
        unsafe { (*self.0.get()).as_mut_ptr().add(index) }
    }

    #[inline(always)]
    pub unsafe fn decrement_handle(&self, index: usize) {
        let cell_ptr = unsafe { self.get_mut_ptr(index) };
        unsafe {
            (*cell_ptr).handle_count.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

pub(crate) static REGISTRY: UnsafeGlobalRegistry =
    UnsafeGlobalRegistry(UnsafeCell::new(Vec::new()));
