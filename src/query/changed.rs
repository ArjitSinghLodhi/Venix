use std::fmt::Display;
use std::marker::PhantomData;

use fxhash::FxHashSet;

use crate::query::filter::Filter;
use crate::query::params::WorldQuery;
use crate::system::validation::{AccessHashSet, AccessVec, FunctionData};
use crate::world::archetypes::{Archetype, ComponentColumn};
use crate::world::storage::CURRENT_FRAME_GENERATION;
use std::any::{Any, TypeId};
use std::sync::RwLock;

#[derive(Debug)]
pub(crate) struct TrackedComponentMeta {
    pub(crate) component_id: TypeId,
    pub(crate) marker_id: TypeId,
    pub(crate) create_marker_column: fn() -> ComponentColumn,
    pub(crate) push_default_marker: unsafe fn(&mut ComponentColumn),
    pub(crate) clear_column_markers: unsafe fn(&mut dyn Any),
}

pub(crate) static TRACKED_COMPONENTS: RwLock<Vec<TrackedComponentMeta>> = RwLock::new(Vec::new());

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
                vec.push(ChangedMarker(0, std::marker::PhantomData));
            },
            clear_column_markers: |raw_any| {
                let vec = raw_any.downcast_mut::<Vec<ChangedMarker<T>>>().unwrap();

                let current_generation =
                    CURRENT_FRAME_GENERATION.load(std::sync::atomic::Ordering::Relaxed);
                let previous_generation = if current_generation == 1 { 2 } else { 1 };

                for marker in vec.iter_mut() {
                    if marker.0 != current_generation && marker.0 != previous_generation {
                        marker.0 = 0;
                    }
                }
            },
        });
    }
}

#[derive(Clone, Copy)]
pub struct ChangedMarker<T>(pub(crate) u8, pub(crate) PhantomData<T>);

pub struct ChangedTracker<T> {
    system_last_generation: u8,
    previous_generation: u8,
    current_generation: u8,
    marker_val: u8,
    _marker: PhantomData<T>,
}

impl<T> ChangedTracker<T> {
    pub fn is_changed(&self) -> bool {
        detect_changed(
            self.marker_val,
            self.current_generation,
            self.system_last_generation,
            self.previous_generation,
        )
    }
}

impl<T: 'static> WorldQuery for ChangedTracker<T> {
    type Item<'w> = ChangedTracker<T>;
    type ReadOnlyItem<'w> = ChangedTracker<T>;
    type Fetch = (u8, u8, u8, *const ChangedMarker<T>);

    fn matches(types: &FxHashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<T>())
    }

    fn collect_access(
        reads: &mut AccessVec<std::any::TypeId>,
        _writes: &mut AccessVec<std::any::TypeId>,
    ) {
        reads.push(TypeId::of::<ChangedMarker<T>>());
        register_tracked_component::<T>();
    }
    unsafe fn init_fetch(archetype: &Archetype, data: &mut FunctionData) -> Self::Fetch {
        let marker_ptr = unsafe { (*archetype.fetch_column_raw::<ChangedMarker<T>>()).as_ptr() };
        let sys_last_gen = data.last_run_generation;
        let current_gen = data.current_run_generation;
        let previous_generation = if current_gen == 1 { 2 } else { 1 };
        (sys_last_gen, previous_generation, current_gen, marker_ptr)
    }
    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
        let marker_ref = unsafe { &(*fetch.3.add(index)) };
        ChangedTracker {
            system_last_generation: fetch.1,
            previous_generation: fetch.1,
            current_generation: fetch.2,
            marker_val: marker_ref.0,
            _marker: PhantomData,
        }
    }

    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
        let marker_ref = unsafe { &(*fetch.3.add(index)) };
        ChangedTracker {
            system_last_generation: fetch.1,
            previous_generation: fetch.1,
            current_generation: fetch.2,
            marker_val: marker_ref.0,
            _marker: PhantomData,
        }
    }
}

pub struct Changed<T>(std::marker::PhantomData<T>);

impl<T: 'static> Filter for Changed<T> {
    fn matches(types: &AccessHashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<T>())
    }
    fn collect_filter(withs: &mut AccessVec<TypeId>, _withouts: &mut AccessVec<TypeId>) {
        withs.push(TypeId::of::<ChangedMarker<T>>());
        register_tracked_component::<T>();
    }

    fn filter_indices(
        archetype: &Archetype,
        indices: &mut Vec<usize>,
        system_data: &mut FunctionData,
    ) {
        let marker_ptr = unsafe { (*archetype.fetch_column_raw::<ChangedMarker<T>>()).as_ptr() };
        let current_generation = system_data.current_run_generation;
        let system_last_generation = system_data.last_run_generation;
        let previous_generation = if current_generation == 1 { 2 } else { 1 };

        indices.retain(|&idx| {
            detect_changed(
                unsafe { (*marker_ptr.add(idx)).0 },
                current_generation,
                system_last_generation,
                previous_generation,
            )
        });
    }
}

pub struct Mut<'w, T> {
    pub(crate) value: *mut T,
    pub(crate) marker: *mut ChangedMarker<T>,
    pub(crate) generation: u8,
    pub(crate) should_modify: bool,
    pub(crate) _marker: std::marker::PhantomData<&'w mut T>,
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
                (*self.marker).0 = self.generation;
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

impl<'w, T: Display> std::fmt::Display for Mut<'w, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { std::fmt::Display::fmt(&*self.value, f) }
    }
}

fn detect_changed(
    marker_val: u8,
    current_generation: u8,
    system_last_generation: u8,
    previous_generation: u8,
) -> bool {
    if marker_val == 0 {
        return false;
    }

    let is_current =
        (marker_val == current_generation) && (system_last_generation != current_generation);
    let is_previous =
        (marker_val == previous_generation) && (system_last_generation != previous_generation);

    is_current || is_previous
}
