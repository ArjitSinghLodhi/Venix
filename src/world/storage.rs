use std::{
    any::{TypeId, type_name},
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{
    commands::commands::{CommandBuffer, ComponentTuple},
    query::changed::TRACKED_COMPONENTS,
    registry::REGISTRY,
    world::archetypes::{ArchetypeId, ArchetypeManager},
};

pub(crate) static CURRENT_FRAME_GENERATION: AtomicU8 = AtomicU8::new(1);
pub struct World {
    pub(crate) archetypes: ArchetypeManager,
    pub(crate) resources: HashMap<std::any::TypeId, std::cell::UnsafeCell<Box<dyn std::any::Any>>>,
    pub(crate) commands: CommandBuffer,
    pub(crate) free_indices_list: Vec<u32>,
}

impl World {
    pub fn new() -> Self {
        let archetypes = ArchetypeManager::new();
        let resources = HashMap::new();
        let world = Self {
            archetypes,
            resources,
            commands: CommandBuffer::new(),
            free_indices_list: Vec::new(),
        };
        world
    }
    pub fn pre_allocate_archetype<T: ComponentTuple>(&mut self) {
        self.get_or_create_archetype_from_generic::<T>();
    }

    pub(crate) fn get_or_create_archetype_from_generic<T: ComponentTuple>(
        &mut self,
    ) -> ArchetypeId {
        self.archetypes.get_or_create_from_generic::<T>()
    }

    pub(crate) fn get_or_create_archetype_from_set(
        &mut self,
        types_set: std::collections::HashSet<std::any::TypeId>,
        types_names_set: HashSet<&'static str>,
    ) -> ArchetypeId {
        self.archetypes
            .get_or_create_from_set(types_set, types_names_set)
    }

    pub fn insert_resource<T: 'static>(&mut self, resource: T) {
        let type_id = std::any::TypeId::of::<T>();
        let boxed_cell = std::cell::UnsafeCell::new(Box::new(resource) as Box<dyn std::any::Any>);
        self.resources.insert(type_id, boxed_cell);
    }

    pub fn remove_resource<T: 'static>(&mut self) -> bool {
        let type_id = TypeId::of::<T>();
        self.resources.remove(&type_id).is_some()
    }

    pub fn get_resource<T: 'static>(&self) -> &T {
        let type_id = TypeId::of::<T>();
        let cell = self.resources.get(&type_id).unwrap_or_else(|| {
            panic!(
                "Requested resource: '{}' was never registered!",
                type_name::<T>()
            );
        });

        unsafe {
            let base_any = &mut *cell.get();
            let casted_ref = base_any
                .downcast_ref::<T>()
                .expect("Resource type mismatch!");
            casted_ref
        }
    }

    pub fn get_resource_mut<T: 'static>(&mut self) -> &mut T {
        let type_id = TypeId::of::<T>();
        let cell = self.resources.get_mut(&type_id).unwrap_or_else(|| {
            panic!(
                "Requested resource: '{}' was never registered!",
                type_name::<T>()
            );
        });
        let base_any = cell.get_mut();
        let casted_mut = base_any
            .downcast_mut::<T>()
            .expect("Resource type mismatch!");
        casted_mut
    }

    pub fn apply_commands(&mut self) {
        let commands_ptr = &mut self.commands as *mut CommandBuffer;
        let commands = unsafe { commands_ptr.as_mut().unwrap() };
        let registry_ptr = std::ptr::addr_of_mut!(REGISTRY);
        let vec = &mut unsafe { &mut *registry_ptr }.0;
        commands.queue.apply(self);
        self.free_indices_list.sort_by(|a, b| b.cmp(a));
        let mut despawn_targets_queue = std::mem::take(&mut commands.pending_despawns);
        for target in despawn_targets_queue.iter() {
            vec[target.entity.registry_index as usize]
                .handle_count
                .fetch_sub(1, Ordering::Relaxed);
        }
        for despawn_target in despawn_targets_queue.drain() {
            despawn_target.apply(self);
        }
        commands.pending_despawns = despawn_targets_queue;
    }
    pub fn clear_changed_tracker(&mut self) {
        let current = CURRENT_FRAME_GENERATION.load(Ordering::Relaxed);
        let next = if current == 1 { 2 } else { 1 };
        CURRENT_FRAME_GENERATION.store(next, Ordering::Relaxed);
        let tracked = TRACKED_COMPONENTS.get().unwrap();

        for archetype in self.archetypes.archetypes.values_mut() {
            unsafe {
                let columns = &mut *archetype.columns.get();

                for meta in tracked.iter() {
                    if let Some(marker_column) = columns.get_mut(&meta.marker_id) {
                        let raw_any = marker_column.data.as_any_mut();
                        (meta.clear_column_markers)(raw_any);
                    }
                }
            }
        }
    }
}
