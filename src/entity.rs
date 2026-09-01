use std::sync::atomic::Ordering;

use crate::registry::REGISTRY;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Entity {
    pub(crate) registry_index: u32,
}

impl Entity {
    pub(crate) fn new(registry_index: u32) -> Self {
        Self { registry_index }
    }
    #[inline(always)]
    pub fn registry_idx(&self) -> u32 {
        self.registry_index
    }
}

impl Clone for Entity {
    fn clone(&self) -> Self {
        unsafe {
            let cell_ptr = REGISTRY.get_ptr(self.registry_idx() as usize);
            (*cell_ptr).handle_count.fetch_add(1, Ordering::Relaxed);
        }
        Entity {
            registry_index: self.registry_index,
        }
    }
}

impl Drop for Entity {
    fn drop(&mut self) {
        unsafe {
            let cell_ptr = REGISTRY.get_ptr(self.registry_idx() as usize);
            (*cell_ptr).handle_count.fetch_sub(1, Ordering::Relaxed);
        }
    }
}
