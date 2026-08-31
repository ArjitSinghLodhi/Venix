use crate::query::QueryData;
use crate::query::QueryFilter;
use crate::reactivity::{TRACKED_COMPONENTS, TrackedComponentMeta};
use crate::system::validation::{AccessHashSet, AccessVec, FunctionData};
use crate::world::archetypes::{Archetype, ComponentColumn};
use crate::world::storage::CurrentBufferIdx;
use fxhash::FxBuildHasher;
use indexmap::IndexSet;
use std::any::TypeId;
use std::marker::PhantomData;

pub(crate) fn register_tracked_component<T: 'static>() {
    let mut tracked = TRACKED_COMPONENTS.write().unwrap();
    let component_id = TypeId::of::<T>();

    if !tracked.iter().any(|m| m.component_id == component_id) {
        tracked.push(TrackedComponentMeta {
            component_id,
            marker_id: TypeId::of::<ChangedMarker<T>>(),
            create_marker_column: || ComponentColumn {
                data: Box::new(Vec::<ChangedMarker<T>>::new()),
            },
            push_default_marker: |column| {
                let raw_any = column.data.as_any_mut();
                let vec = raw_any.downcast_mut::<Vec<ChangedMarker<T>>>().unwrap();
                vec.push(ChangedMarker {
                    markers: [false; 2],
                    phantom: PhantomData,
                });
            },
            clear_column_markers: |raw_any| {
                let idx = CurrentBufferIdx::current_write_idx();
                let vec = raw_any.downcast_mut::<Vec<ChangedMarker<T>>>().unwrap();
                vec.iter_mut().for_each(|marker| {
                    marker.markers[idx as usize] = false;
                });
            },
        });
    }
}

#[derive(Clone, Copy)]
pub struct ChangedMarker<T> {
    markers: [bool; 2],
    phantom: PhantomData<T>,
}

pub struct ChangedTracker<T> {
    changed: bool,
    _marker: PhantomData<T>,
}

impl<T> ChangedTracker<T> {
    #[inline(always)]
    pub fn is_changed(&self) -> bool {
        self.changed
    }
}

impl<T: 'static> QueryData for ChangedTracker<T> {
    type Item<'w> = ChangedTracker<T>;
    type ReadOnlyItem<'w> = ChangedTracker<T>;
    type Fetch = (u8, *const ChangedMarker<T>);

    fn matches(types: &IndexSet<TypeId, FxBuildHasher>) -> bool {
        types.contains(&TypeId::of::<ChangedMarker<T>>())
    }

    fn collect_access(
        reads: &mut AccessVec<std::any::TypeId>,
        _writes: &mut AccessVec<std::any::TypeId>,
    ) {
        reads.push(TypeId::of::<ChangedMarker<T>>());
        register_tracked_component::<T>();
    }

    unsafe fn init_fetch(archetype: &Archetype, _data: &mut FunctionData) -> Self::Fetch {
        let marker_ptr = unsafe { (*archetype.fetch_column_raw::<ChangedMarker<T>>()).as_ptr() };
        let current_read_idx = CurrentBufferIdx::current_read_idx();

        (current_read_idx, marker_ptr)
    }

    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
        let marker_ref = unsafe { &(*fetch.1.add(index)) };
        let changed = marker_ref.markers[fetch.0 as usize];
        ChangedTracker {
            changed,
            _marker: PhantomData,
        }
    }

    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
        let marker_ref = unsafe { &(*fetch.1.add(index)) };
        let changed = marker_ref.markers[fetch.0 as usize];
        ChangedTracker {
            changed,
            _marker: PhantomData,
        }
    }
}
pub struct Changed<T>(std::marker::PhantomData<T>);
impl<T: 'static> QueryFilter for Changed<T> {
    fn matches(types: &AccessHashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<ChangedMarker<T>>())
    }

    fn collect_filter(withs: &mut AccessVec<TypeId>, _withouts: &mut AccessVec<TypeId>) {
        withs.push(TypeId::of::<ChangedMarker<T>>());
        register_tracked_component::<T>();
    }

    fn filter_indices(
        archetype: &Archetype,
        indices: &mut Vec<usize>,
        _system_data: &mut FunctionData,
    ) {
        let marker_ptr = unsafe { (*archetype.fetch_column_raw::<ChangedMarker<T>>()).as_ptr() };
        let current_read_idx = CurrentBufferIdx::current_read_idx();

        indices.retain(|&idx| unsafe { &*marker_ptr.add(idx) }.markers[current_read_idx as usize]);
    }
}

pub struct Mut<'w, T> {
    pub(crate) value: *mut T,
    pub(crate) marker: *mut ChangedMarker<T>,
    pub(crate) current_write_idx: u8,
    pub(crate) should_modify: bool,
    pub(crate) _marker: std::marker::PhantomData<&'w mut T>,
}

impl<'w, T> Mut<'w, T> {
    #[inline(always)]
    pub fn into_raw_mut(self) -> &'w mut T {
        unsafe { &mut *self.value }
    }
    #[inline(always)]
    pub fn bypass_change_detection(&mut self) -> &mut T {
        unsafe { &mut *self.value }
    }

    #[inline(always)]
    pub fn trigger_change_detection(&mut self) {
        if self.should_modify {
            unsafe {
                (*self.marker).markers[self.current_write_idx as usize] = true;
            }
        }
    }

    #[inline(always)]
    pub fn as_ref(&self) -> &T {
        unsafe { &*self.value }
    }
}

unsafe impl<'w, T> Send for Mut<'w, T> {}
unsafe impl<'w, T> Sync for Mut<'w, T> {}

impl<'w, T> std::ops::Deref for Mut<'w, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.value }
    }
}

impl<'w, T> std::ops::DerefMut for Mut<'w, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            if self.should_modify {
                (*self.marker).markers[self.current_write_idx as usize] = true;
            }
            &mut *self.value
        }
    }
}

impl<'w, T: std::fmt::Debug> std::fmt::Debug for Mut<'w, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { std::fmt::Debug::fmt(&*self.value, f) }
    }
}

impl<'w, T: std::fmt::Display> std::fmt::Display for Mut<'w, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { std::fmt::Display::fmt(&*self.value, f) }
    }
}
