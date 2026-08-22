use std::{
    any::{TypeId, type_name},
    borrow::Borrow,
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
    hash::{BuildHasher, Hash, Hasher},
};

use crate::{
    commands::commands::ComponentTuple, entity::Entity, query::changed::TRACKED_COMPONENTS,
};

pub(crate) trait AnyColumn: std::any::Any + Send + Sync {
    unsafe fn swap_remove_erased(&mut self, idx: usize);
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    unsafe fn move_row_erased(&mut self, index: usize, dst: *mut dyn AnyColumn);
    fn clone_empty(&self) -> Box<dyn AnyColumn>;
}

impl<T: 'static + Send + Sync> AnyColumn for Vec<T> {
    unsafe fn swap_remove_erased(&mut self, idx: usize) {
        self.swap_remove(idx);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    unsafe fn move_row_erased(&mut self, index: usize, dst: *mut dyn AnyColumn) {
        let item = self.swap_remove(index);
        let dst_vec = unsafe { &mut *(dst as *mut Vec<T>) };
        dst_vec.push(item);
    }
    fn clone_empty(&self) -> Box<dyn AnyColumn> {
        Box::new(Vec::<T>::new())
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub struct ArchetypeId(u32);

impl ArchetypeId {
    pub(crate) fn new(id: u32) -> ArchetypeId {
        ArchetypeId(id)
    }
    pub(crate) fn id(&self) -> u32 {
        self.0
    }
}

pub struct ComponentColumn {
    pub(crate) data: Box<dyn AnyColumn>,
}

pub struct Archetype {
    pub(crate) id: ArchetypeId,
    pub(crate) types: std::collections::HashSet<std::any::TypeId>,
    pub(crate) entities: Vec<Entity>,
    pub(crate) columns: UnsafeCell<HashMap<TypeId, ComponentColumn>>,
    pub(crate) type_names: HashSet<&'static str>,
}

unsafe impl Sync for Archetype {}
unsafe impl Send for Archetype {}

impl Archetype {
    pub(crate) fn new(
        id: ArchetypeId,
        types: std::collections::HashSet<std::any::TypeId>,
        columns: std::collections::HashMap<std::any::TypeId, ComponentColumn>,
        type_names: HashSet<&'static str>,
    ) -> Self {
        Self {
            id,
            types,
            entities: Vec::new(),
            columns: std::cell::UnsafeCell::new(columns),
            type_names: type_names,
        }
    }

    pub(crate) unsafe fn fetch_column_raw<T: 'static>(&self) -> *mut Vec<T> {
        unsafe {
            let cols = &mut *self.columns.get();
            let col = cols
                .get_mut(&std::any::TypeId::of::<T>())
                .unwrap_or_else(|| panic!("Column missing of: {:?}", type_name::<T>()));
            col.data
                .as_any_mut()
                .downcast_mut::<Vec<T>>()
                .expect("Type mismatch!") as *mut Vec<T>
        }
    }
    pub(crate) fn id(&self) -> u32 {
        self.id.0
    }
}

#[derive(Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct ComponentIdSlice([TypeId]);

impl From<&[TypeId]> for &ComponentIdSlice {
    fn from(slice: &[TypeId]) -> Self {
        unsafe { &*(slice as *const [TypeId] as *const ComponentIdSlice) }
    }
}

impl Borrow<ComponentIdSlice> for Box<[TypeId]> {
    fn borrow(&self) -> &ComponentIdSlice {
        (&**self).into()
    }
}

#[derive(Default)]
pub(crate) struct ArchetypeManager {
    index: HashMap<u64, ArchetypeId>,
    pub(crate) archetypes: HashMap<ArchetypeId, Archetype>,
    next_id: u32,
}

impl ArchetypeManager {
    pub(crate) fn new() -> Self {
        Self {
            index: HashMap::new(),
            archetypes: HashMap::new(),
            next_id: 0,
        }
    }
    pub(crate) fn find_id_by_combining(
        &self,
        old_id: ArchetypeId,
        incoming_ids: &[TypeId],
    ) -> Option<ArchetypeId> {
        let old_arch = self.archetypes.get(&old_id)?;
        let hasher_builder = self.index.hasher();
        let mut combined_hash: u64 = 0;

        for type_id in &old_arch.types {
            let mut state = hasher_builder.build_hasher();
            type_id.hash(&mut state);
            combined_hash ^= state.finish();
        }

        for type_id in incoming_ids {
            let mut state = hasher_builder.build_hasher();
            type_id.hash(&mut state);
            combined_hash ^= state.finish();
        }

        self.index.get(&combined_hash).copied()
    }
    pub(crate) fn find_id_by_subtracting(
        &self,
        old_id: ArchetypeId,
        removed_ids: &[TypeId],
    ) -> Option<ArchetypeId> {
        let old_arch = self.archetypes.get(&old_id)?;
        let hasher_builder = self.index.hasher();
        let mut combined_hash: u64 = 0;

        for type_id in &old_arch.types {
            let mut state = hasher_builder.build_hasher();
            type_id.hash(&mut state);
            combined_hash ^= state.finish();
        }
        for type_id in removed_ids {
            let mut state = hasher_builder.build_hasher();
            type_id.hash(&mut state);
            combined_hash ^= state.finish();
        }

        self.index.get(&combined_hash).copied()
    }
    pub(crate) fn find_id_from_slice(&self, types_slice: &[TypeId]) -> Option<ArchetypeId> {
        let mut order_independent_hash: u64 = 0;
        let hasher_builder = self.index.hasher();

        for type_id in types_slice {
            let mut state = hasher_builder.build_hasher();
            type_id.hash(&mut state);
            order_independent_hash ^= state.finish();
        }

        self.index.get(&order_independent_hash).copied()
    }
    pub(crate) fn get_or_create_from_set(
        &mut self,
        types_set: HashSet<TypeId>,
        types_names_set: HashSet<&'static str>,
    ) -> ArchetypeId {
        let mut order_independent_hash: u64 = 0;
        let hasher_builder = self.index.hasher();

        for type_id in &types_set {
            let mut state = hasher_builder.build_hasher();
            type_id.hash(&mut state);
            order_independent_hash ^= state.finish();
        }

        if let Some(id) = self.index.get(&order_independent_hash).copied() {
            return id;
        }

        let new_id = ArchetypeId::new(self.next_id);
        self.next_id += 1;

        let columns = HashMap::new();
        let new_arch = Archetype::new(new_id, types_set, columns, types_names_set);

        self.index.insert(order_independent_hash, new_id);
        self.archetypes.insert(new_id, new_arch);

        new_id
    }

    pub(crate) fn get_or_create_from_generic<T: ComponentTuple>(&mut self) -> ArchetypeId {
        let incoming_ids = T::get_type_ids();
        if let Some(id) = self.find_id_from_slice(incoming_ids) {
            return id;
        }
        let incoming_names = T::get_type_names();
        let names_ref = incoming_names.as_ref();

        let mut types_set = HashSet::with_capacity(incoming_ids.len());
        for &id in incoming_ids {
            types_set.insert(id);
        }

        let mut types_names_set = HashSet::with_capacity(names_ref.len());
        for &name in names_ref {
            types_names_set.insert(name);
        }

        let mut order_independent_hash: u64 = 0;
        let hasher_builder = self.index.hasher();

        for type_id in &types_set {
            let mut state = hasher_builder.build_hasher();
            type_id.hash(&mut state);
            order_independent_hash ^= state.finish();
        }

        let new_id = ArchetypeId(self.next_id);
        self.next_id += 1;

        let mut columns = HashMap::new();
        T::create_empty_columns(&mut columns);
        {
            let tracked = TRACKED_COMPONENTS.read().unwrap();
            let mut injections = Vec::new();

            for meta in tracked.iter() {
                if columns.contains_key(&meta.component_id) {
                    injections.push((meta.marker_id, (meta.create_marker_column)()));
                }
            }

            for (marker_id, column_wrapper) in injections {
                columns.insert(marker_id, column_wrapper);
                types_set.insert(marker_id);
            }
        }
        let new_arch = Archetype::new(new_id, types_set, columns, types_names_set);

        self.index.insert(order_independent_hash, new_id);
        self.archetypes.insert(new_id, new_arch);

        new_id
    }

    pub(crate) fn get_mut(&mut self, id: ArchetypeId) -> Option<&mut Archetype> {
        self.archetypes.get_mut(&id)
    }

    pub(crate) fn get(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(&id)
    }
}
