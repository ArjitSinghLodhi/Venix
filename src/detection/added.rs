use std::{any::TypeId, marker::PhantomData};

use fxhash::FxBuildHasher;
use indexmap::IndexSet;

use crate::{
    detection::{TRACKED_COMPONENTS, TrackedComponentMeta},
    extensions::ComponentColumn,
    query::{filter::QueryFilter, query::QueryData},
    world::storage::CurrentBufferIdx,
};

pub(crate) fn register_added_tracked_component<T: 'static>() {
    let mut tracked = TRACKED_COMPONENTS.write().unwrap();
    if !tracked
        .iter()
        .any(|meta| meta.component_id == TypeId::of::<T>())
    {
        tracked.push(TrackedComponentMeta {
            component_id: TypeId::of::<T>(),
            marker_id: TypeId::of::<AddedMarker<T>>(),
            create_marker_column: || ComponentColumn {
                data: Box::new(Vec::<AddedMarker<T>>::new()),
            },
            push_default_marker: |column| {
                let raw_any = column.data.as_any_mut();
                let vec = raw_any.downcast_mut::<Vec<AddedMarker<T>>>().unwrap();
                let current_read_idx = CurrentBufferIdx::current_write_idx();
                let mut added_marker = [false; 2];
                added_marker[current_read_idx as usize] = true;
                vec.push(AddedMarker {
                    added_marker,
                    phantom: PhantomData,
                });
            },
            clear_column_markers: |raw_any| {
                let idx = CurrentBufferIdx::current_write_idx();
                let vec = raw_any.downcast_mut::<Vec<AddedMarker<T>>>().unwrap();
                vec.iter_mut().for_each(|marker| {
                    marker.added_marker[idx as usize] = false;
                });
            },
        });
    }
}

pub struct AddedMarker<T> {
    added_marker: [bool; 2],
    phantom: PhantomData<T>,
}

pub struct Added<T>(PhantomData<T>);

impl<T: 'static> QueryFilter for Added<T> {
    fn matches(types: &crate::extensions::AccessHashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<AddedMarker<T>>())
    }
    fn collect_filter(
        withs: &mut crate::extensions::AccessVec<std::any::TypeId>,
        _withouts: &mut crate::extensions::AccessVec<std::any::TypeId>,
    ) {
        withs.push(TypeId::of::<AddedMarker<T>>());
        register_added_tracked_component::<T>();
    }
    fn filter_indices(
        archetype: &crate::extensions::Archetype,
        indices: &mut Vec<usize>,
        _system_data: &mut crate::extensions::FunctionData,
    ) {
        let marker_ptr = unsafe { (*archetype.fetch_column_raw::<AddedMarker<T>>()).as_ptr() };
        let current_read_idx = CurrentBufferIdx::current_read_idx();
        indices.retain(|idx| {
            unsafe { &*marker_ptr.add(*idx) }.added_marker[current_read_idx as usize]
        });
    }
}

pub struct AddedTracker<T> {
    added: bool,
    _phantom: PhantomData<T>,
}

impl<T: 'static> AddedTracker<T> {
    pub fn is_added(&self) -> bool {
        self.added
    }
}

impl<T: 'static> QueryData for AddedTracker<T> {
    type Item<'w> = AddedTracker<T>;
    type ReadOnlyItem<'w> = AddedTracker<T>;
    type Fetch = (u8, *const AddedMarker<T>);
    fn collect_access(
        reads: &mut crate::extensions::AccessVec<std::any::TypeId>,
        _writes: &mut crate::extensions::AccessVec<std::any::TypeId>,
    ) {
        reads.push(TypeId::of::<AddedMarker<T>>());
        register_added_tracked_component::<T>();
    }
    fn matches(types: &IndexSet<TypeId, FxBuildHasher>) -> bool {
        types.contains(&TypeId::of::<AddedMarker<T>>())
    }
    unsafe fn init_fetch(
        archetype: &crate::extensions::Archetype,
        _systems_data: &mut crate::extensions::FunctionData,
    ) -> Self::Fetch {
        let marker_ptr = unsafe { (*archetype.fetch_column_raw::<AddedMarker<T>>()).as_ptr() };
        let current_read_idx = CurrentBufferIdx::current_read_idx();
        (current_read_idx, marker_ptr)
    }

    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
        let marker_ref = unsafe { &*fetch.1.add(index) };
        let added = marker_ref.added_marker[fetch.0 as usize];
        AddedTracker {
            added,
            _phantom: PhantomData,
        }
    }

    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
        let marker_ref = unsafe { &(*fetch.1.add(index)) };
        let added = marker_ref.added_marker[fetch.0 as usize];
        AddedTracker {
            added,
            _phantom: PhantomData,
        }
    }
}
