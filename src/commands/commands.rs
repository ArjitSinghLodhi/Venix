use std::{
    any::TypeId,
    cell::UnsafeCell,
    ptr::NonNull,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use indexmap::{IndexMap, IndexSet};
use thread_local::ThreadLocal;

use crate::{
    commands::{
        bundle::ComponentBundle,
        command_queue::{CommandQueue, WorldCommand},
    },
    entity::Entity,
    registry::{REGISTRY, RegistryCell},
    system::validation::{FunctionData, ParamAccess, SystemParam},
    world::{
        archetypes::{AnyColumn, Archetype, ArchetypeId, ComponentColumn},
        storage::World,
    },
};

#[cfg(feature = "reactivity")]
use crate::detection::TRACKED_COMPONENTS;

pub(crate) struct SpawnCommand<T: ComponentBundle> {
    pub(crate) components: T,
}

impl<T: ComponentBundle + Send> WorldCommand for SpawnCommand<T> {
    fn apply(self, world: &mut World) {
        let arch_id = world.archetypes_manager.get_or_create_from_generic::<T>();
        let next_idx = match world.archetypes_manager.get(arch_id) {
            Some(arch) => arch.entities.len() as u32,
            None => 0,
        };
        let assigned_registry_idx = alloc_registry_cell(arch_id, next_idx, world);
        let arch = world
            .archetypes_manager
            .get_mut(arch_id)
            .expect("Archetype generation failed");

        arch.entities.push(Entity::new(assigned_registry_idx));
        self.components.push_to_archetype(arch);
        #[cfg(feature = "reactivity")]
        unsafe {
            let columns = &mut *arch.columns.get();
            initialize_spawn_markers(columns);
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct DespawnCommand {
    pub(crate) entity: Entity,
}

impl DespawnCommand {
    pub fn apply(self, world: &mut World) {
        let target_registry_idx = self.entity.registry_index as usize;
        let (arch_id, target_idx) = unsafe {
            let cell_ptr = REGISTRY.get_ptr(target_registry_idx);
            let cell_arch_id = (*cell_ptr).archetype_id;
            if (*cell_ptr)
                .handle_count
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
            {
                let types_names = &&world
                    .archetypes_manager
                    .archetypes
                    .get(&(*cell_ptr).archetype_id)
                    .unwrap()
                    .type_names;
                panic!(
                    "Safety Violation: Attempted to despawn an Entity while active handles are still held! of archetype:\n{:?}",
                    types_names
                );
            }

            (cell_arch_id, (*cell_ptr).idx)
        };

        let arch = world
            .archetypes_manager
            .get_mut(arch_id)
            .expect("Target archetype missing");
        let last_idx = (arch.entities.len() - 1) as u32;

        if target_idx != last_idx {
            let swapped_entity_registry_idx =
                arch.entities[last_idx as usize].registry_index as usize;

            unsafe {
                let swapped_cell_ptr = REGISTRY.get_mut_ptr(swapped_entity_registry_idx);
                (*swapped_cell_ptr).idx = target_idx;
            }
        }
        world.free_indices_list.push(target_registry_idx as u32);

        arch.entities.swap_remove(target_idx as usize);

        unsafe {
            let cols = &mut *arch.columns.get();
            for col in cols.values_mut() {
                col.data.swap_remove_erased(target_idx as usize);
            }
        }
    }

    pub fn despawn_target(&self) -> &Entity {
        &self.entity
    }
}

pub(crate) struct AddComponentsCommand<T: ComponentBundle> {
    pub(crate) entity: Entity,
    pub(crate) components: T,
}

impl<T: ComponentBundle + Send> WorldCommand for AddComponentsCommand<T> {
    fn apply(self, world: &mut World) {
        let target_registry_idx = self.entity.registry_index as usize;
        let (old_arch_id, old_idx) = get_registry_location(target_registry_idx);
        let incoming_ids = T::get_type_ids();

        let new_arch_id = if let Some(id) = world
            .archetypes_manager
            .find_target_id_for_addition(old_arch_id, incoming_ids)
        {
            id
        } else {
            create_addition_archetype::<T>(world, old_arch_id, incoming_ids)
        };

        if old_arch_id == new_arch_id {
            return;
        }

        unsafe {
            let (old_arch, new_arch) = get_double_archetypes(world, old_arch_id, new_arch_id);
            {
                let new_cols = &mut *new_arch.columns.get();
                let old_cols = &mut *old_arch.columns.get();
                if new_cols.is_empty() {
                    T::create_empty_columns(new_cols);
                    clone_existing_columns(old_cols, new_cols);
                    #[cfg(feature = "reactivity")]
                    initialize_missing_archetype_markers(&new_arch.types, new_cols);
                }
                move_matching_columns(old_cols, new_cols, old_idx as usize);
            }
            let new_dense_idx = new_arch.entities.len() as u32;
            self.components.push_to_archetype(new_arch);
            #[cfg(feature = "reactivity")]
            {
                let fresh_new_cols = &mut *new_arch.columns.get();
                migrate_addition_markers(&old_arch.types, incoming_ids, fresh_new_cols);
            }
            swap_remove_entity_registry_update(old_arch, old_idx);
            let entity_handle = old_arch.entities.swap_remove(old_idx as usize);
            new_arch.entities.push(entity_handle);
            update_registry_cell(target_registry_idx, new_arch_id, new_dense_idx);
        }
    }
}

pub struct InsertComponentsCommand<T: ComponentBundle> {
    pub entity: Entity,
    pub components: T,
}

impl<T: ComponentBundle + Send> WorldCommand for InsertComponentsCommand<T> {
    fn apply(self, world: &mut World) {
        let target_registry_idx = self.entity.registry_index as usize;
        let (old_arch_id, old_idx) = get_registry_location(target_registry_idx);
        let incoming_ids = T::get_type_ids();

        let new_arch_id = if let Some(id) = world
            .archetypes_manager
            .find_target_id_for_addition(old_arch_id, incoming_ids)
        {
            id
        } else {
            create_addition_archetype::<T>(world, old_arch_id, incoming_ids)
        };

        if old_arch_id == new_arch_id {
            let arch = world
                .archetypes_manager
                .get_mut(old_arch_id)
                .expect("Venix Engine Fatal: Entity registry pointed to an untracked Archetype ID");

            unsafe {
                self.components.insert_to_archetype(arch, old_idx as usize);
            }
            return;
        }

        unsafe {
            let (old_arch, new_arch) = get_double_archetypes(world, old_arch_id, new_arch_id);
            let new_dense_idx = new_arch.entities.len() as u32;

            {
                let new_cols = &mut *new_arch.columns.get();
                let old_cols = &mut *old_arch.columns.get();

                if new_cols.is_empty() {
                    T::create_empty_columns(new_cols);
                    clone_existing_columns(old_cols, new_cols);
                    #[cfg(feature = "reactivity")]
                    initialize_missing_archetype_markers(&new_arch.types, new_cols);
                }

                move_matching_columns(old_cols, new_cols, old_idx as usize);
            }
            self.components
                .insert_to_archetype(new_arch, new_dense_idx as usize);

            #[cfg(feature = "reactivity")]
            {
                let fresh_new_cols = &mut *new_arch.columns.get();
                migrate_addition_markers(&old_arch.types, incoming_ids, fresh_new_cols);
            }

            swap_remove_entity_registry_update(old_arch, old_idx);
            let entity_handle = old_arch.entities.swap_remove(old_idx as usize);
            new_arch.entities.push(entity_handle);
            update_registry_cell(target_registry_idx, new_arch_id, new_dense_idx);
        }
    }
}

pub(crate) struct RemoveComponentsCommand<T: ComponentBundle> {
    pub(crate) entity: Entity,
    pub(crate) _marker: std::marker::PhantomData<T>,
}

impl<T: ComponentBundle + Send> WorldCommand for RemoveComponentsCommand<T> {
    fn apply(self, world: &mut World) {
        let target_registry_idx = self.entity.registry_index as usize;
        let (old_arch_id, old_idx) = get_registry_location(target_registry_idx);
        let removed_ids = T::get_type_ids();

        let new_arch_id = if let Some(id) = world
            .archetypes_manager
            .find_target_id_for_subtraction(old_arch_id, removed_ids)
        {
            id
        } else {
            create_subtraction_archetype::<T>(world, old_arch_id, removed_ids)
        };

        if old_arch_id == new_arch_id {
            return;
        }

        unsafe {
            let (old_arch, new_arch) = get_double_archetypes(world, old_arch_id, new_arch_id);
            let old_cols = &mut *old_arch.columns.get();
            let new_cols = &mut *new_arch.columns.get();
            if new_cols.is_empty() {
                populate_subtracted_columns(&new_arch.types, old_cols, new_cols);
            }
            let new_dense_idx = new_arch.entities.len() as u32;
            move_matching_columns(old_cols, new_cols, old_idx as usize);
            erase_subtracted_columns(removed_ids, old_cols, old_idx as usize);

            #[cfg(feature = "reactivity")]
            erase_subtracted_markers(&old_arch.types, &new_arch.types, old_cols, old_idx as usize);

            swap_remove_entity_registry_update(old_arch, old_idx);

            let entity_handle = old_arch.entities.swap_remove(old_idx as usize);
            new_arch.entities.push(entity_handle);

            update_registry_cell(target_registry_idx, new_arch_id, new_dense_idx);
        }
    }
}

#[inline(always)]
fn get_registry_location(registry_idx: usize) -> (ArchetypeId, u32) {
    unsafe {
        let cell_ptr = REGISTRY.get_ptr(registry_idx);
        let arch_id = (*cell_ptr).archetype_id;
        (arch_id, (*cell_ptr).idx)
    }
}

#[inline(always)]
fn update_registry_cell(registry_idx: usize, archetype_id: ArchetypeId, dense_idx: u32) {
    unsafe {
        let cell_ptr = REGISTRY.get_mut_ptr(registry_idx);
        (*cell_ptr).archetype_id = archetype_id;
        (*cell_ptr).idx = dense_idx;
    }
}

fn alloc_registry_cell(archetype_id: ArchetypeId, dense_idx: u32, world: &mut World) -> u32 {
    let registry_ptr = &REGISTRY;
    if let Some(recycled_idx) = world.free_indices_list.pop() {
        unsafe {
            let cell_ptr = REGISTRY.get_mut_ptr(recycled_idx as usize);
            (*cell_ptr) = RegistryCell {
                archetype_id,
                idx: dense_idx,
                handle_count: std::sync::atomic::AtomicU32::new(0),
            };
        }
        recycled_idx
    } else {
        let len = (*registry_ptr).len() as u32;
        (*registry_ptr).push(RegistryCell {
            archetype_id,
            idx: dense_idx,
            handle_count: std::sync::atomic::AtomicU32::new(0),
        });
        len
    }
}

#[inline(always)]
unsafe fn get_double_archetypes(
    world: &mut World,
    old_id: ArchetypeId,
    new_id: ArchetypeId,
) -> (&mut Archetype, &mut Archetype) {
    let map_ptr =
        &mut world.archetypes_manager.archetypes as *mut FxHashMap<ArchetypeId, Archetype>;
    let old_arch = unsafe { (*map_ptr).get_mut(&old_id).expect("Old archetype missing") };
    let new_arch = unsafe { (*map_ptr).get_mut(&new_id).expect("New archetype missing") };
    (old_arch, new_arch)
}

fn create_addition_archetype<T: ComponentBundle>(
    world: &mut World,
    old_arch_id: ArchetypeId,
    incoming_ids: &[TypeId],
) -> ArchetypeId {
    let old_arch = world
        .archetypes_manager
        .archetypes
        .get(&old_arch_id)
        .unwrap();
    let mut new_types = old_arch.types.clone();
    for id in incoming_ids {
        new_types.insert(*id);
    }

    #[cfg(feature = "reactivity")]
    world
        .archetypes_manager
        .sync_tracking_markers(&mut new_types);

    let mut new_types_names = old_arch.type_names.clone();
    for id in T::get_type_names().as_ref() {
        new_types_names.insert(id);
    }
    world
        .archetypes_manager
        .get_or_create_from_set(new_types, new_types_names)
}

fn create_subtraction_archetype<T: ComponentBundle>(
    world: &mut World,
    old_arch_id: ArchetypeId,
    removed_ids: &[TypeId],
) -> ArchetypeId {
    let old_arch = world
        .archetypes_manager
        .archetypes
        .get(&old_arch_id)
        .unwrap();
    let mut new_types = old_arch.types.clone();
    for id in removed_ids {
        new_types.swap_remove(id);
    }

    #[cfg(feature = "reactivity")]
    world
        .archetypes_manager
        .sync_tracking_markers(&mut new_types);

    let mut new_types_names = old_arch.type_names.clone();
    for id_name in T::get_type_names().as_ref() {
        new_types_names.shift_remove(id_name);
    }
    world
        .archetypes_manager
        .get_or_create_from_set(new_types, new_types_names)
}

#[inline(always)]
fn clone_existing_columns(
    src: &IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
    dst: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
) {
    for (type_id, old_col) in src.iter() {
        if !dst.contains_key(type_id) {
            dst.insert(
                *type_id,
                ComponentColumn {
                    data: old_col.data.clone_empty(),
                },
            );
        }
    }
}

#[inline(always)]
fn populate_subtracted_columns(
    allowed_types: &IndexSet<TypeId, FxBuildHasher>,
    src: &IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
    dst: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
) {
    for (type_id, old_col) in src.iter() {
        if allowed_types.contains(type_id) {
            dst.insert(
                *type_id,
                ComponentColumn {
                    data: old_col.data.clone_empty(),
                },
            );
        }
    }
}

#[inline(always)]
unsafe fn move_matching_columns(
    src: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
    dst: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
    row_idx: usize,
) {
    for (type_id, old_col) in src.iter_mut() {
        if let Some(new_col) = dst.get_mut(type_id) {
            let dst_ptr = &mut *new_col.data as *mut dyn AnyColumn;
            unsafe { old_col.data.move_row_erased(row_idx, dst_ptr) };
        }
    }
}

#[inline(always)]
unsafe fn erase_subtracted_columns(
    ids: &[TypeId],
    columns: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
    row_idx: usize,
) {
    for id in ids {
        if let Some(col) = columns.get_mut(id) {
            unsafe { col.data.swap_remove_erased(row_idx) };
        }
    }
}

#[inline(always)]
unsafe fn swap_remove_entity_registry_update(arch: &mut Archetype, removed_row_idx: u32) {
    let last_idx = (arch.entities.len() - 1) as u32;
    if removed_row_idx != last_idx {
        let swapped_entity_registry_idx = arch.entities[last_idx as usize].registry_index as usize;
        let swapped_cell = unsafe { REGISTRY.get_mut_ptr(swapped_entity_registry_idx) };
        unsafe {
            (*swapped_cell).idx = removed_row_idx;
        }
    }
}

#[cfg(feature = "reactivity")]
#[inline(always)]
fn initialize_spawn_markers(columns: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>) {
    let tracked = TRACKED_COMPONENTS.read().unwrap();
    for meta in tracked.iter() {
        if let Some(marker_column) = columns.get_mut(&meta.marker_id) {
            unsafe { (meta.push_default_marker)(marker_column) };
        }
    }
}

#[cfg(feature = "reactivity")]
#[inline(always)]
fn initialize_missing_archetype_markers(
    types: &IndexSet<TypeId, FxBuildHasher>,
    columns: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
) {
    let tracked = TRACKED_COMPONENTS.read().unwrap();
    for meta in tracked.iter() {
        if types.contains(&meta.marker_id) && !columns.contains_key(&meta.marker_id) {
            columns.insert(meta.marker_id, (meta.create_marker_column)());
        }
    }
}

#[cfg(feature = "reactivity")]
#[inline(always)]
unsafe fn migrate_addition_markers(
    old_types: &IndexSet<TypeId, FxBuildHasher>,
    incoming_ids: &[TypeId],
    new_cols: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
) {
    let tracked = TRACKED_COMPONENTS.read().unwrap();
    for meta in tracked.iter() {
        if new_cols.contains_key(&meta.marker_id) {
            if old_types.contains(&meta.marker_id) {
                continue;
            } else if incoming_ids.contains(&meta.component_id)
                && let Some(marker_column) = new_cols.get_mut(&meta.marker_id)
            {
                unsafe { (meta.push_default_marker)(marker_column) };
            }
        }
    }
}

#[cfg(feature = "reactivity")]
#[inline(always)]
unsafe fn erase_subtracted_markers(
    old_types: &IndexSet<TypeId, FxBuildHasher>,
    new_types: &IndexSet<TypeId, FxBuildHasher>,
    old_cols: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>,
    row_idx: usize,
) {
    let tracked = TRACKED_COMPONENTS.read().unwrap();
    for meta in tracked.iter() {
        if old_types.contains(&meta.marker_id)
            && !new_types.contains(&meta.marker_id)
            && let Some(marker_col) = old_cols.get_mut(&meta.marker_id)
        {
            unsafe { marker_col.data.swap_remove_erased(row_idx) };
        }
    }
}

pub(crate) struct CommandsBufferData {
    pub(crate) queue: CommandQueue,
    pub(crate) despawns: FxHashSet<DespawnCommand>,
}

impl CommandsBufferData {
    pub(crate) fn new() -> Self {
        Self {
            queue: CommandQueue::new(),
            despawns: FxHashSet::default(),
        }
    }
}
pub(crate) struct LocalSlot {
    pub(crate) is_busy: AtomicBool,
    pub(crate) data: UnsafeCell<CommandsBufferData>,
}
pub(crate) struct CommandBuffer {
    pub(crate) data: RwLock<CommandsBufferData>,
    pub(crate) local_data: ThreadLocal<LocalSlot>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(CommandsBufferData::new()),
            local_data: ThreadLocal::new(),
        }
    }
}

pub(crate) enum CommandsOrigin {
    ThreadLocal(*const AtomicBool),
    HeapFallback(NonNull<CommandsBufferData>),
}

pub struct Commands<'a> {
    pub(crate) local_data: NonNull<CommandsBufferData>,
    pub(crate) origin: CommandsOrigin,
    pub(crate) master_buffer: Arc<CommandBuffer>,
    pub(crate) _marker: std::marker::PhantomData<&'a mut CommandsBufferData>,
}

impl Commands<'_> {
    fn push<C: WorldCommand + 'static>(&mut self, command: C) {
        unsafe {
            self.local_data.as_mut().queue.push(command);
        }
    }

    pub fn spawn<B: ComponentBundle + Send>(&mut self, components: B) {
        self.push(SpawnCommand { components });
    }

    pub fn despawn(&mut self, entity: Entity) {
        unsafe {
            self.local_data
                .as_mut()
                .despawns
                .insert(DespawnCommand { entity });
        }
    }

    pub fn add_components<C: ComponentBundle + Send>(&mut self, entity: Entity, components: C) {
        self.push(AddComponentsCommand { entity, components });
    }

    pub fn remove_components<C: ComponentBundle + Send>(&mut self, entity: Entity) {
        self.push(RemoveComponentsCommand::<C> {
            entity,
            _marker: std::marker::PhantomData,
        });
    }

    pub fn insert_components<C: ComponentBundle + Send>(&mut self, entity: Entity, components: C) {
        self.push(InsertComponentsCommand { entity, components });
    }

    pub fn despawn_iter<F>(&self, mut f: F)
    where
        F: for<'b> FnMut(&'b Entity),
    {
        let buff = self.master_buffer.data.read().unwrap();
        for cmd in buff.despawns.iter() {
            f(cmd.despawn_target())
        }
    }

    pub fn despawn_iter_local<F>(&self, mut f: F)
    where
        F: for<'b> FnMut(&'b Entity),
    {
        unsafe {
            for cmd in self.local_data.as_ref().despawns.iter() {
                f(cmd.despawn_target())
            }
        }
    }
    pub(crate) fn push_fn<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World) + Send + 'static,
    {
        unsafe {
            self.local_data.as_mut().queue.push_fn(f);
        }
    }

    pub fn insert_resource<T: 'static + Send>(&mut self, resource: T) {
        self.push_fn(|world| world.insert_resource(resource));
    }

    pub fn remove_resource<T: 'static + Send>(&mut self) {
        self.push_fn(|world| {
            world.remove_resource::<T>();
        });
    }
}

impl<'a> SystemParam for Commands<'a> {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, _data: &mut FunctionData) -> Self {
        let master_buffer_ptr = Arc::clone(&world.commands);
        let master_ref = &master_buffer_ptr;
        let slot = master_ref.local_data.get_or(|| LocalSlot {
            is_busy: AtomicBool::new(false),
            data: UnsafeCell::new(CommandsBufferData::new()),
        });
        if slot
            .is_busy
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            Self {
                local_data: unsafe { NonNull::new_unchecked(slot.data.get()) },
                origin: CommandsOrigin::ThreadLocal(&slot.is_busy as *const AtomicBool),
                master_buffer: master_buffer_ptr,
                _marker: std::marker::PhantomData,
            }
        } else {
            let heap_box = Box::new(CommandsBufferData::new());
            let heap_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(heap_box)) };

            Self {
                local_data: heap_ptr,
                origin: CommandsOrigin::HeapFallback(heap_ptr),
                master_buffer: master_buffer_ptr,
                _marker: std::marker::PhantomData,
            }
        }
    }
}

impl<'a> Drop for Commands<'a> {
    fn drop(&mut self) {
        unsafe {
            let data_ref = self.local_data.as_mut();
            if !data_ref.queue.is_empty() || !data_ref.despawns.is_empty() {
                let mut master_data = self.master_buffer.data.write().unwrap();

                if !data_ref.queue.is_empty() {
                    master_data.queue.merge(&mut data_ref.queue);
                    data_ref.queue.clear_bytes();
                }

                if !data_ref.despawns.is_empty() {
                    master_data.despawns.extend(data_ref.despawns.drain());
                }
            }
            match self.origin {
                CommandsOrigin::ThreadLocal(is_busy_flag) => {
                    (*is_busy_flag).store(false, Ordering::Relaxed);
                }
                CommandsOrigin::HeapFallback(heap_ptr) => {
                    let _cleanup = Box::from_raw(heap_ptr.as_ptr());
                }
            }
        }
    }
}
