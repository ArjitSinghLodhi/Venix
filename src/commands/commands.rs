use std::{
    any::TypeId,
    cell::{RefCell, RefMut},
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use thread_local::ThreadLocal;

use crate::{
    commands::command_queue::{CommandQueue, WorldCommand},
    entity::Entity,
    query::changed::TRACKED_COMPONENTS,
    registry::{REGISTRY, RegistryCell},
    system::validation::{FunctionData, ParamAccess, SystemParam},
    world::{
        archetypes::{AnyColumn, Archetype, ArchetypeId, ComponentColumn},
        storage::World,
    },
};

pub trait ComponentTuple: Send + Sync + 'static {
    const TYPE_IDS: &[TypeId];
    fn get_type_ids() -> &'static [TypeId];
    fn push_to_archetype(self, archetype: &mut Archetype);
    fn create_empty_columns(columns: &mut HashMap<TypeId, ComponentColumn>);

    type NamesArray: AsRef<[&'static str]>;
    fn get_type_names() -> Self::NamesArray;
}

macro_rules! impl_component_tuple {
    ($($T:ident),*) => {
        impl<$($T: 'static + Send + Sync),*> ComponentTuple for ($($T,)*) {

            const TYPE_IDS: &[TypeId] = &[ $( TypeId::of::<$T>() ),* ];

            fn get_type_ids() -> &'static [TypeId] {
                Self::TYPE_IDS
            }

            fn create_empty_columns(columns: &mut HashMap<TypeId, ComponentColumn>) {
                $(
                    let id = TypeId::of::<$T>();
                    columns.insert(id, ComponentColumn {
                        data: Box::new(Vec::<$T>::new()),
                    });
                )*
            }

            fn push_to_archetype(self, archetype: &mut Archetype) {
                #[allow(non_snake_case)]
                let ($($T,)*) = self;
                unsafe {
                    $(
                        let vec_ptr = archetype.fetch_column_raw::<$T>();
                        if !vec_ptr.is_null() {
                            (*vec_ptr).push($T);
                        }
                    )*
                }
            }

            type NamesArray = [&'static str; 0 $( + { let _ = stringify!($T); 1 } )*];

            #[inline(always)]
            fn get_type_names() -> Self::NamesArray {
                [ $( std::any::type_name::<$T>() ),* ]
            }
        }
    };
}

impl_component_tuple!(A);
impl_component_tuple!(A, B);
impl_component_tuple!(A, B, C);
impl_component_tuple!(A, B, C, D);
impl_component_tuple!(A, B, C, D, E);
impl_component_tuple!(A, B, C, D, E, F);
impl_component_tuple!(A, B, C, D, E, F, G);
impl_component_tuple!(A, B, C, D, E, F, G, H);
impl_component_tuple!(A, B, C, D, E, F, G, H, I);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);

pub(crate) struct SpawnCommand<T: ComponentTuple> {
    pub(crate) components: T,
}

impl<T: ComponentTuple> WorldCommand for SpawnCommand<T> {
    fn apply(self, world: &mut World) {
        let arch_id = world.archetypes_manager.get_or_create_from_generic::<T>();
        let arch = world
            .archetypes_manager
            .get_mut(arch_id)
            .expect("Archetype generation failed");
        let next_idx = arch.entities.len();

        let registry_ptr = std::ptr::addr_of_mut!(REGISTRY);

        let assigned_registry_idx = if let Some(recycled_idx) = world.free_indices_list.pop() {
            unsafe {
                let vec = &mut (*registry_ptr).0;
                vec[recycled_idx as usize] = RegistryCell {
                    archetype_id: arch_id,
                    idx: next_idx as u32,
                    handle_count: std::sync::atomic::AtomicU32::new(0),
                };
            }
            recycled_idx
        } else {
            unsafe {
                let vec = &mut (*registry_ptr).0;
                let len = vec.len() as u32;
                vec.push(RegistryCell {
                    archetype_id: arch_id,
                    idx: next_idx as u32,
                    handle_count: std::sync::atomic::AtomicU32::new(0),
                });
                len
            }
        };

        arch.entities.push(Entity::new(assigned_registry_idx));
        self.components.push_to_archetype(arch);
        unsafe {
            let columns = &mut *arch.columns.get();
            let tracked = TRACKED_COMPONENTS.get().unwrap();

            for meta in tracked.iter() {
                if let Some(marker_column) = columns.get_mut(&meta.marker_id) {
                    (meta.push_default_marker)(marker_column);
                }
            }
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
        let registry_ptr = std::ptr::addr_of_mut!(REGISTRY);

        let (arch_id, target_idx) = unsafe {
            let vec = &mut (*registry_ptr).0;
            let cell = &vec[target_registry_idx];

            if cell.handle_count.load(std::sync::atomic::Ordering::Relaxed) > 0 {
                let types_names = &&world
                    .archetypes_manager
                    .get(cell.archetype_id)
                    .unwrap()
                    .type_names;
                panic!(
                    "Safety Violation: Attempted to despawn an Entity while active handles are still held! of archetype:\n{:?}",
                    types_names
                );
            }

            (cell.archetype_id, cell.idx)
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
                let vec = &mut (*registry_ptr).0;
                let swapped_cell = &mut vec[swapped_entity_registry_idx];
                swapped_cell.idx = target_idx;
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

pub(crate) struct AddComponentsCommand<T: ComponentTuple> {
    pub(crate) entity: Entity,
    pub(crate) components: T,
}

impl<T: ComponentTuple> WorldCommand for AddComponentsCommand<T> {
    fn apply(self, world: &mut World) {
        let target_registry_idx = self.entity.registry_index as usize;
        let registry_ptr = std::ptr::addr_of_mut!(REGISTRY);

        let (old_arch_id, old_idx) = unsafe {
            let vec = &mut (*registry_ptr).0;
            let cell = &vec[target_registry_idx];
            (cell.archetype_id, cell.idx)
        };

        let incoming_ids = T::get_type_ids();

        let new_arch_id = if let Some(id) = world
            .archetypes_manager
            .find_id_by_combining(old_arch_id, incoming_ids)
        {
            id
        } else {
            let old_arch = world
                .archetypes_manager
                .archetypes
                .get(&old_arch_id)
                .unwrap();
            let mut new_types = old_arch.types.clone();
            for id in incoming_ids {
                new_types.insert(*id);
            }

            let mut new_types_names = old_arch.type_names.clone();
            for id in T::get_type_names().as_ref() {
                new_types_names.insert(id);
            }

            world.get_or_create_archetype_from_set(new_types, new_types_names)
        };

        if old_arch_id == new_arch_id {
            return;
        }

        unsafe {
            let map_ptr = &mut world.archetypes_manager.archetypes
                as *mut std::collections::HashMap<ArchetypeId, Archetype>;
            let old_arch = (*map_ptr)
                .get_mut(&old_arch_id)
                .expect("Old archetype missing");
            let new_arch = (*map_ptr)
                .get_mut(&new_arch_id)
                .expect("New archetype missing");

            let old_cols = &mut *old_arch.columns.get();
            let new_cols = &mut *new_arch.columns.get();

            if new_cols.is_empty() {
                T::create_empty_columns(new_cols);
                for (type_id, old_col) in old_cols.iter() {
                    if !new_cols.contains_key(type_id) {
                        new_cols.insert(
                            *type_id,
                            ComponentColumn {
                                data: old_col.data.clone_empty(),
                            },
                        );
                    }
                }
            }
            let new_dense_idx = new_arch.entities.len() as u32;
            for (type_id, old_col) in old_cols.iter_mut() {
                if let Some(new_col) = new_cols.get_mut(type_id) {
                    let dst_ptr = &mut *new_col.data as *mut dyn AnyColumn;
                    old_col.data.move_row_erased(old_idx as usize, dst_ptr);
                }
            }
            let columns = &mut *new_arch.columns.get();
            let tracked = TRACKED_COMPONENTS.get().unwrap();

            self.components.push_to_archetype(new_arch);
            for meta in tracked.iter() {
                if incoming_ids.contains(&meta.marker_id) {
                    if let Some(marker_column) = columns.get_mut(&meta.marker_id) {
                        (meta.push_default_marker)(marker_column);
                    }
                }
            }
            let last_idx = (old_arch.entities.len() - 1) as u32;
            if old_idx != last_idx {
                let swapped_entity_registry_idx =
                    old_arch.entities[last_idx as usize].registry_index as usize;
                let vec = &mut (*registry_ptr).0;
                let swapped_cell = &mut vec[swapped_entity_registry_idx];
                swapped_cell.idx = old_idx;
            }

            let entity_handle = old_arch.entities.swap_remove(old_idx as usize);
            new_arch.entities.push(entity_handle);

            let vec = &mut (*registry_ptr).0;
            let target_cell = &mut vec[target_registry_idx];
            target_cell.archetype_id = new_arch_id;
            target_cell.idx = new_dense_idx;
        }
    }
}

pub(crate) struct RemoveComponentsCommand<T: ComponentTuple> {
    pub(crate) entity: crate::entity::Entity,
    pub(crate) _marker: std::marker::PhantomData<T>,
}

impl<T: ComponentTuple> WorldCommand for RemoveComponentsCommand<T> {
    fn apply(self, world: &mut World) {
        let target_registry_idx = self.entity.registry_index as usize;
        let registry_ptr = std::ptr::addr_of_mut!(REGISTRY);

        let (old_arch_id, old_idx) = unsafe {
            let vec = &mut (*registry_ptr).0;
            let cell = &vec[target_registry_idx];
            (cell.archetype_id, cell.idx)
        };

        let removed_ids = T::get_type_ids();

        let new_arch_id = if let Some(id) = world
            .archetypes_manager
            .find_id_by_subtracting(old_arch_id, removed_ids)
        {
            id
        } else {
            let old_arch = world
                .archetypes_manager
                .archetypes
                .get(&old_arch_id)
                .unwrap();
            let mut new_types = old_arch.types.clone();
            for id in removed_ids {
                new_types.remove(id);
            }

            let mut new_types_names = old_arch.type_names.clone();
            for id_name in T::get_type_names().as_ref() {
                new_types_names.remove(*id_name);
            }

            world.get_or_create_archetype_from_set(new_types, new_types_names)
        };

        if old_arch_id == new_arch_id {
            return;
        }

        unsafe {
            let map_ptr = &mut world.archetypes_manager.archetypes
                as *mut std::collections::HashMap<ArchetypeId, Archetype>;

            let old_arch = (*map_ptr)
                .get_mut(&old_arch_id)
                .expect("Old archetype missing");
            let new_arch = (*map_ptr)
                .get_mut(&new_arch_id)
                .expect("New archetype missing");

            let old_cols = &mut *old_arch.columns.get();
            let new_cols = &mut *new_arch.columns.get();

            if new_cols.is_empty() {
                for (type_id, old_col) in old_cols.iter() {
                    if !removed_ids.contains(type_id) {
                        new_cols.insert(
                            *type_id,
                            ComponentColumn {
                                data: old_col.data.clone_empty(),
                            },
                        );
                    }
                }
            }

            let new_dense_idx = new_arch.entities.len() as u32;
            for (type_id, new_col) in new_cols.iter_mut() {
                if let Some(old_col) = old_cols.get_mut(type_id) {
                    let dst_ptr = &mut *new_col.data as *mut dyn AnyColumn;
                    old_col.data.move_row_erased(old_idx as usize, dst_ptr);
                }
            }

            let tracked = TRACKED_COMPONENTS.get().unwrap();

            for id in removed_ids {
                if let Some(old_col) = old_cols.get_mut(id) {
                    old_col.data.swap_remove_erased(old_idx as usize);
                }
                if let Some(meta) = tracked.iter().find(|m| m.marker_id == *id) {
                    if let Some(marker_col) = old_cols.get_mut(&meta.marker_id) {
                        marker_col.data.swap_remove_erased(old_idx as usize);
                    }
                }
            }
            let last_idx = (old_arch.entities.len() - 1) as u32;
            if old_idx != last_idx {
                let swapped_entity_registry_idx =
                    old_arch.entities[last_idx as usize].registry_index as usize;
                let vec = &mut (*registry_ptr).0;
                let swapped_cell = &mut vec[swapped_entity_registry_idx];
                swapped_cell.idx = old_idx;
            }

            let entity_handle = old_arch.entities.swap_remove(old_idx as usize);
            new_arch.entities.push(entity_handle);

            let vec = &mut (*registry_ptr).0;
            let target_cell_mut = &mut vec[target_registry_idx];
            target_cell_mut.archetype_id = new_arch_id;
            target_cell_mut.idx = new_dense_idx;
        }
    }
}

pub(crate) struct CommandBuffer {
    pub(crate) queue: CommandQueue,
    pub(crate) pending_despawns: HashSet<DespawnCommand>,
    pub(crate) local_channels: ThreadLocal<RefCell<CommandQueue>>,
    pub(crate) local_despawns: ThreadLocal<RefCell<HashSet<DespawnCommand>>>,
    pub(crate) merge_lock: Mutex<()>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            queue: CommandQueue::new(),
            pending_despawns: HashSet::new(),
            local_channels: ThreadLocal::new(),
            local_despawns: ThreadLocal::new(),
            merge_lock: Mutex::new(()),
        }
    }
}

pub struct Commands<'a> {
    pub(crate) local_queue: RefMut<'a, CommandQueue>,
    pub(crate) local_despawns: RefMut<'a, HashSet<DespawnCommand>>,
    pub(crate) master_buffer_address: usize,
}

impl<'a> SystemParam for Commands<'a> {
    fn get_access() -> ParamAccess {
        let mut access = ParamAccess::default();
        access.commands_accessed.push(TypeId::of::<Commands>());
        access
    }

    fn extract(world: &mut World, _data: &mut FunctionData) -> Self {
        unsafe {
            let master_buffer_ptr = std::ptr::addr_of_mut!(world.commands);
            let master_buffer_address = master_buffer_ptr as usize;

            let master_ref = &mut *master_buffer_ptr;

            let q_cell = master_ref
                .local_channels
                .get_or(|| RefCell::new(CommandQueue::new()));

            let d_cell = master_ref
                .local_despawns
                .get_or(|| RefCell::new(HashSet::new()));

            Self {
                local_queue: q_cell.borrow_mut(),
                local_despawns: d_cell.borrow_mut(),
                master_buffer_address,
            }
        }
    }
}

impl Commands<'_> {
    fn push<C: WorldCommand + 'static>(&mut self, command: C) {
        self.local_queue.push(command);
    }

    pub fn spawn<T: ComponentTuple>(&mut self, components: T) {
        self.push(SpawnCommand { components });
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.local_despawns.insert(DespawnCommand { entity });
    }

    pub fn add_components<C: ComponentTuple>(&mut self, entity: Entity, components: C) {
        self.push(AddComponentsCommand { entity, components });
    }

    pub fn remove_components<T: ComponentTuple>(&mut self, entity: Entity) {
        self.push(RemoveComponentsCommand::<T> {
            entity,
            _marker: std::marker::PhantomData,
        });
    }

    pub fn despawn_iter(&self) -> std::collections::hash_set::Iter<'_, DespawnCommand> {
        self.local_despawns.iter()
    }
}

impl<'a> Drop for Commands<'a> {
    fn drop(&mut self) {
        if self.local_queue.is_empty() && self.local_despawns.is_empty() {
            return;
        }

        unsafe {
            let master_buffer_ptr = self.master_buffer_address as *mut CommandBuffer;
            let _guard = (*master_buffer_ptr).merge_lock.lock().unwrap();

            if !self.local_queue.is_empty() {
                let queue_offset = std::mem::offset_of!(CommandBuffer, queue);
                let queue_ptr =
                    (master_buffer_ptr as *mut u8).add(queue_offset) as *mut CommandQueue;

                (*queue_ptr).merge(&mut self.local_queue);
                self.local_queue.clear_bytes();
            }

            if !self.local_despawns.is_empty() {
                let despawns_offset = std::mem::offset_of!(CommandBuffer, pending_despawns);
                let despawns_ptr = (master_buffer_ptr as *mut u8).add(despawns_offset)
                    as *mut HashSet<DespawnCommand>;

                (*despawns_ptr).extend(self.local_despawns.drain());
            }
        }
    }
}
