use std::marker::PhantomData;

use crate::world::archetypes::ComponentColumn;
use crate::world::storage::CURRENT_FRAME_GENERATION;
use std::any::TypeId;
use std::sync::OnceLock;

#[derive(Debug)]
pub(crate) struct TrackedComponentMeta {
    pub(crate) component_id: TypeId,
    pub(crate) marker_id: TypeId,
    pub(crate) create_marker_column: fn() -> ComponentColumn,
    pub(crate) push_default_marker: unsafe fn(&mut ComponentColumn),
    pub(crate) clear_column_markers: unsafe fn(&mut dyn std::any::Any),
}

pub(crate) static TRACKED_COMPONENTS: OnceLock<Vec<TrackedComponentMeta>> = OnceLock::new();

pub(crate) fn register_tracked_component<T: 'static + Send + Sync>() {
    let static_ptr = &TRACKED_COMPONENTS as *const OnceLock<Vec<TrackedComponentMeta>>
        as *mut OnceLock<Vec<TrackedComponentMeta>>;
    let tracked = unsafe { (*static_ptr).get_mut().unwrap() };

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

pub struct Changed<T>(std::marker::PhantomData<T>);

pub struct Mut<'w, T> {
    pub(crate) value: *mut T,
    pub(crate) marker: *mut ChangedMarker<T>,
    pub(crate) deref_mut_function: fn(*mut ChangedMarker<T>, u8),
    pub(crate) generation: u8,
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
            (self.deref_mut_function)(self.marker, self.generation);
            &mut *self.value
        }
    }
}

impl<'w, T: std::fmt::Debug> std::fmt::Debug for Mut<'w, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { std::fmt::Debug::fmt(&*self.value, f) }
    }
}

pub(crate) fn no_op_mut<T>(_marker: *mut ChangedMarker<T>, _generation: u8) {}
pub(crate) fn changed_marker_modify<T>(marker: *mut ChangedMarker<T>, generation: u8) {
    (unsafe { &mut *marker }).0 = generation;
}
