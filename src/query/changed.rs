use crate::query::filter::QueryFilter;
use crate::query::query::QueryData;
use crate::system::validation::{AccessHashSet, AccessVec, FunctionData, FunctionGenerationData};
use crate::world::archetypes::{Archetype, ComponentColumn};
use crate::world::storage::GenerationRing;
use fxhash::FxHashSet;
use std::any::{Any, TypeId};
use std::marker::PhantomData;
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
                vec.push(ChangedMarker(0, 0, std::marker::PhantomData));
            },
            clear_column_markers: |raw_any| {
                let vec = raw_any.downcast_mut::<Vec<ChangedMarker<T>>>().unwrap();
                let current_generation = GenerationRing::current();
                let stale_generation = GenerationRing::stale_threshold(current_generation);

                for marker in vec.iter_mut() {
                    if marker.0 == stale_generation {
                        marker.0 = 0;
                        marker.1 = 0;
                    }
                }
            },
        });
    }
}

#[derive(Clone, Copy)]
pub struct ChangedMarker<T>(pub(crate) u8, pub(crate) u32, pub(crate) PhantomData<T>);

pub struct ChangedTracker<T> {
    system_last_generation: u8,
    previous_generation: u8,
    current_generation: u8,
    marker_val: u8,
    author_system_id: u32,
    reading_system_id: u32,
    _marker: PhantomData<T>,
}

impl<T> ChangedTracker<T> {
    #[inline(always)]
    pub fn is_changed(&self) -> bool {
        detect_changed(
            self.marker_val,
            self.author_system_id,
            self.current_generation,
            self.system_last_generation,
            self.previous_generation,
            self.reading_system_id,
        )
    }
}

impl<T: 'static> QueryData for ChangedTracker<T> {
    type Item<'w> = ChangedTracker<T>;
    type ReadOnlyItem<'w> = ChangedTracker<T>;
    type Fetch = (u8, u8, u8, u32, *const ChangedMarker<T>);

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
        let generation_data = data
            .get_data::<FunctionGenerationData>()
            .expect("Couldn't find FunctionGenerationData");

        let sys_last_gen = generation_data.last_run_generation;
        let reading_system_id = generation_data.system_id;

        let current_gen = GenerationRing::current();
        let previous_generation = GenerationRing::previous(current_gen);

        (
            sys_last_gen,
            previous_generation,
            current_gen,
            reading_system_id,
            marker_ptr,
        )
    }

    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
        let marker_ref = unsafe { &(*fetch.4.add(index)) };
        ChangedTracker {
            system_last_generation: fetch.0,
            previous_generation: fetch.1,
            current_generation: fetch.2,
            reading_system_id: fetch.3,
            marker_val: marker_ref.0,
            author_system_id: marker_ref.1,
            _marker: PhantomData,
        }
    }

    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
        let marker_ref = unsafe { &(*fetch.4.add(index)) };
        ChangedTracker {
            system_last_generation: fetch.0,
            previous_generation: fetch.1,
            current_generation: fetch.2,
            reading_system_id: fetch.3,
            marker_val: marker_ref.0,
            author_system_id: marker_ref.1,
            _marker: PhantomData,
        }
    }
}
pub struct Changed<T>(std::marker::PhantomData<T>);
impl<T: 'static> QueryFilter for Changed<T> {
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
        let generation_data = system_data.get_data::<FunctionGenerationData>().unwrap();

        let current_generation = GenerationRing::current();
        let system_last_generation = generation_data.last_run_generation;
        let previous_generation = GenerationRing::previous(current_generation);

        let reading_system_id = generation_data.system_id;

        indices.retain(|&idx| {
            let marker = unsafe { &*marker_ptr.add(idx) };
            detect_changed(
                marker.0,
                marker.1,
                current_generation,
                system_last_generation,
                previous_generation,
                reading_system_id,
            )
        });
    }
}

pub struct Mut<'w, T> {
    pub(crate) value: *mut T,
    pub(crate) marker: *mut ChangedMarker<T>,
    pub(crate) generation: u8,
    pub(crate) system_id: u32,
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
                *self.marker =
                    ChangedMarker(self.generation, self.system_id, std::marker::PhantomData);
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

pub fn detect_changed(
    marker_val: u8,
    author_system_id: u32,
    current_generation: u8,
    system_last_generation: u8,
    previous_generation: u8,
    reading_system_id: u32,
) -> bool {
    if marker_val == 0 {
        return false;
    }
    if marker_val == current_generation {
        if system_last_generation == current_generation {
            return false;
        }
        return reading_system_id > author_system_id;
    }
    if marker_val == previous_generation {
        if system_last_generation == previous_generation {
            return reading_system_id < author_system_id;
        }

        let two_generations_ago = GenerationRing::stale_threshold(current_generation);
        return system_last_generation == two_generations_ago;
    }

    false
}
