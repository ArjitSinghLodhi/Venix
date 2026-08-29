use fxhash::FxHashMap;
use std::{
    any::{TypeId, type_name},
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{
    commands::{CommandBuffer, DespawnCommand, ParallelCommands, bundle::ComponentBundle},
    events::{ParallelEventReader, ParallelEventWriter, TRACKED_EVENTS},
    extensions::{FunctionData, SystemParam},
    registry::REGISTRY,
    world::archetypes::{ArchetypeId, ArchetypeManager},
};

#[cfg(feature = "reactivity")]
use crate::detection::TRACKED_COMPONENTS;

pub(crate) struct CurrentBufferIdx;
static CURRENT_BUFFER_IDX: AtomicU8 = AtomicU8::new(1);

impl CurrentBufferIdx {
    #[inline]
    #[cfg(feature = "reactivity")]
    pub(crate) fn current_read_idx() -> u8 {
        CURRENT_BUFFER_IDX.load(Ordering::Relaxed)
    }
    #[cfg(feature = "reactivity")]
    pub(crate) fn current_write_idx() -> u8 {
        let idx = CURRENT_BUFFER_IDX.load(Ordering::Relaxed);
        if idx == 0 { 1 } else { 0 }
    }

    pub(crate) fn advance() {
        let idx = CURRENT_BUFFER_IDX.load(Ordering::Relaxed);
        let next_idx = if idx == 1 { 0 } else { 1 };
        CURRENT_BUFFER_IDX.store(next_idx, Ordering::Relaxed);
    }
}

pub struct World {
    pub(crate) archetypes_manager: ArchetypeManager,
    pub(crate) resources:
        FxHashMap<std::any::TypeId, std::cell::UnsafeCell<Box<dyn std::any::Any>>>,
    pub(crate) commands: CommandBuffer,
    pub(crate) free_indices_list: Vec<u32>,
}

impl World {
    pub(crate) fn new() -> Self {
        Self {
            archetypes_manager: ArchetypeManager::new(),
            resources: FxHashMap::default(),
            commands: CommandBuffer::new(),
            free_indices_list: Vec::new(),
        }
    }
    pub fn pre_allocate_archetype<T: ComponentBundle>(&mut self) {
        self.get_or_create_archetype_from_generic::<T>();
    }

    pub(crate) fn get_or_create_archetype_from_generic<T: ComponentBundle>(
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
        let commands = std::mem::replace(&mut self.commands, CommandBuffer::new());
        {
            commands.queue.write().unwrap().apply(self);
            let despawns = commands.despawns.pin_owned();
            for target in despawns.iter() {
                unsafe {
                    REGISTRY.decrement_handle(target.entity.registry_index as usize);
                }
            }

            for despawn_target_ref in despawns.iter() {
                unsafe {
                    let despawn_target =
                        std::ptr::read(despawn_target_ref as *const DespawnCommand);
                    despawns.remove(&despawn_target);
                    despawn_target.apply(self);
                }
            }
        }
        self.free_indices_list.sort_by(|a, b| b.cmp(a));
        self.commands = commands;
    }

    pub(crate) fn end_of_frame_sync(&mut self) {
        CurrentBufferIdx::advance();
        #[cfg(feature = "reactivity")]
        self.clear_changed_tracker();
        self.clear_events();
    }

    #[cfg(feature = "reactivity")]
    fn clear_changed_tracker(&mut self) {
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

    fn clear_events(&mut self) {
        let tracked_events = TRACKED_EVENTS.read().unwrap();
        for meta in tracked_events.iter() {
            let unsafecell = self
                .resources
                .get_mut(&meta.event_id)
                .expect("Registered event Not initialized somehow? maybe removed");
            (meta.clear_events)(unsafecell);
        }
    }

    pub(crate) fn get_par_commands(&mut self) -> ParallelCommands {
        ParallelCommands::extract(self, &mut FunctionData::new())
    }

    pub(crate) fn get_par_event_writer<T: 'static + Send + Sync>(
        &mut self,
    ) -> ParallelEventWriter<T> {
        ParallelEventWriter::extract(self, &mut FunctionData::new())
    }

    pub(crate) fn get_par_event_reader<T: 'static + Send + Sync>(
        &mut self,
    ) -> ParallelEventReader<T> {
        ParallelEventReader::extract(self, &mut FunctionData::new())
    }
}
