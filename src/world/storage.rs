use fxhash::FxHashMap;
use std::{
    any::{TypeId, type_name},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
};

use crate::{
    commands::commands::{CommandBuffer, ComponentTuple},
    query::changed::TRACKED_COMPONENTS,
    registry::REGISTRY,
    world::archetypes::{ArchetypeId, ArchetypeManager},
};

pub(crate) static CURRENT_FRAME_GENERATION: AtomicU8 = AtomicU8::new(1);
pub struct World {
    pub archetypes_manager: ArchetypeManager,
    pub(crate) resources:
        FxHashMap<std::any::TypeId, std::cell::UnsafeCell<Box<dyn std::any::Any>>>,
    pub(crate) commands: Arc<CommandBuffer>,
    pub(crate) free_indices_list: Vec<u32>,
}

impl World {
    pub(crate) fn new() -> Self {
        Self {
            archetypes_manager: ArchetypeManager::new(),
            resources: FxHashMap::default(),
            commands: Arc::new(CommandBuffer::new()),
            free_indices_list: Vec::new(),
        }
    }
    pub fn pre_allocate_archetype<T: ComponentTuple>(&mut self) {
        self.get_or_create_archetype_from_generic::<T>();
    }

    pub(crate) fn get_or_create_archetype_from_generic<T: ComponentTuple>(
        &mut self,
    ) -> ArchetypeId {
        self.archetypes_manager.get_or_create_from_generic::<T>()
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
            base_any
                .downcast_ref::<T>()
                .expect("Resource type mismatch!")
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
        base_any
            .downcast_mut::<T>()
            .expect("Resource type mismatch!")
    }
    pub fn get_resource_opt<T: 'static>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let cell = self.resources.get(&type_id)?;

        unsafe {
            let base_any = &mut *cell.get();
            let casted_ref = base_any.downcast_ref::<T>()?;
            Some(casted_ref)
        }
    }

    pub fn get_resource_mut_opt<T: 'static>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        let cell = self.resources.get_mut(&type_id)?;
        let base_any = cell.get_mut();
        let casted_mut = base_any.downcast_mut::<T>()?;
        Some(casted_mut)
    }
    pub fn apply_commands(&mut self) {
        let commands = std::mem::replace(&mut self.commands, CommandBuffer::new().into());

        commands.data.write().unwrap().queue.apply(self);

        for target in commands.data.write().unwrap().despawns.iter() {
            unsafe {
                REGISTRY.decrement_handle(target.entity.registry_index as usize);
            }
        }

        for despawn_target in commands.data.write().unwrap().despawns.drain() {
            despawn_target.apply(self);
        }

        self.free_indices_list.sort_by(|a, b| b.cmp(a));
        self.commands = commands;
    }

    pub fn clear_changed_tracker(&mut self) {
        let current = CURRENT_FRAME_GENERATION.load(Ordering::Relaxed);
        let next = if current == 1 { 2 } else { 1 };
        CURRENT_FRAME_GENERATION.store(next, Ordering::Relaxed);
        let tracked = TRACKED_COMPONENTS.read().unwrap();

        for archetype in self.archetypes_manager.archetypes.values_mut() {
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
