use std::{any::TypeId, collections::HashSet, sync::atomic::Ordering};

use crate::{
    entity::Entity,
    query::changed::{ChangedMarker, Mut, changed_marker_modify, no_op_mut},
    system::validation::{AccessVec, FunctionData},
    world::{archetypes::Archetype, storage::CURRENT_FRAME_GENERATION},
};

pub trait WorldQuery {
    type Item<'w>;
    type ReadOnlyItem<'w>;
    type Fetch: Copy;

    fn matches(types: &HashSet<TypeId>) -> bool;
    unsafe fn init_fetch(archetype: &Archetype, systems_data: &mut FunctionData) -> Self::Fetch;
    fn collect_access(
        reads: &mut AccessVec<std::any::TypeId>,
        writes: &mut AccessVec<std::any::TypeId>,
    );
    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w>;
    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w>;
}

impl<T: 'static> WorldQuery for &T {
    type Item<'w> = &'w T;
    type ReadOnlyItem<'w> = &'w T;
    type Fetch = *const T;

    fn matches(types: &HashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<T>())
    }
    fn collect_access(
        reads: &mut AccessVec<std::any::TypeId>,
        _writes: &mut AccessVec<std::any::TypeId>,
    ) {
        reads.push(TypeId::of::<T>());
    }
    unsafe fn init_fetch(archetype: &Archetype, _: &mut FunctionData) -> Self::Fetch {
        unsafe { (*archetype.fetch_column_raw::<T>()).as_ptr() }
    }
    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
        unsafe { &*fetch.add(index) }
    }
    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
        unsafe { &*fetch.add(index) }
    }
}
impl<T: 'static + Send + Sync> WorldQuery for &mut T {
    type Item<'w> = Mut<'w, T>;
    type ReadOnlyItem<'w> = &'w T;
    type Fetch = (
        *mut T,
        *mut ChangedMarker<T>,
        u8,
        fn(*mut ChangedMarker<T>, generation: u8),
    );

    fn matches(types: &HashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<T>())
    }

    unsafe fn init_fetch(archetype: &Archetype, _: &mut FunctionData) -> Self::Fetch {
        unsafe {
            let data_ptr = (*archetype.fetch_column_raw::<T>()).as_mut_ptr();
            let columns = &mut *archetype.columns.get();
            let marker_id = TypeId::of::<ChangedMarker<T>>();
            let current_generation = CURRENT_FRAME_GENERATION.load(Ordering::Relaxed);

            if let Some(column) = columns.get_mut(&marker_id) {
                let vec_ptr = column
                    .data
                    .as_any_mut()
                    .downcast_mut::<Vec<ChangedMarker<T>>>()
                    .unwrap();

                (
                    data_ptr,
                    vec_ptr.as_mut_ptr(),
                    current_generation,
                    changed_marker_modify,
                )
            } else {
                (
                    data_ptr,
                    std::ptr::null_mut(),
                    current_generation,
                    no_op_mut,
                )
            }
        }
    }

    #[inline(always)]
    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
        unsafe {
            Mut {
                value: fetch.0.add(index),
                marker: fetch.1.add(index),
                deref_mut_function: fetch.3,
                generation: fetch.2,
                _marker: std::marker::PhantomData,
            }
        }
    }

    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
        unsafe { &*fetch.0.add(index) }
    }

    fn collect_access(_reads: &mut AccessVec<TypeId>, writes: &mut AccessVec<TypeId>) {
        writes.push(TypeId::of::<T>());
        writes.push(TypeId::of::<ChangedMarker<T>>());
    }
}

impl WorldQuery for Entity {
    type Item<'w> = &'w Entity;
    type ReadOnlyItem<'w> = &'w Entity;
    type Fetch = *const Entity;

    fn matches(_types: &HashSet<TypeId>) -> bool {
        true
    }
    fn collect_access(_reads: &mut AccessVec<TypeId>, _writes: &mut AccessVec<TypeId>) {}
    unsafe fn init_fetch(archetype: &Archetype, _: &mut FunctionData) -> Self::Fetch {
        archetype.entities.as_ptr()
    }
    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
        unsafe { &*fetch.add(index) }
    }
    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
        unsafe { &*fetch.add(index) }
    }
}

impl<T: 'static> WorldQuery for Option<&T> {
    type Item<'w> = Option<&'w T>;
    type ReadOnlyItem<'w> = Option<&'w T>;
    type Fetch = Option<*const T>;

    fn matches(_types: &HashSet<TypeId>) -> bool {
        true
    }

    fn collect_access(reads: &mut AccessVec<TypeId>, _writes: &mut AccessVec<TypeId>) {
        reads.push(TypeId::of::<T>());
    }

    unsafe fn init_fetch(archetype: &Archetype, _: &mut FunctionData) -> Self::Fetch {
        if archetype.types.contains(&TypeId::of::<T>()) {
            unsafe { Some((*archetype.fetch_column_raw::<T>()).as_mut_ptr()) }
        } else {
            None
        }
    }
    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
        if let Some(fetch) = fetch {
            unsafe { Some(&*fetch.add(index)) }
        } else {
            None
        }
    }

    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
        if let Some(fetch) = fetch {
            unsafe { Some(&*fetch.add(index)) }
        } else {
            None
        }
    }
}

impl<T: 'static + Send + Sync> WorldQuery for Option<&mut T> {
    type Item<'w> = Option<Mut<'w, T>>;
    type ReadOnlyItem<'w> = Option<&'w T>;
    type Fetch = (
        Option<*mut T>,
        *mut ChangedMarker<T>,
        u8,
        fn(*mut ChangedMarker<T>, u8),
    );

    fn matches(_types: &HashSet<TypeId>) -> bool {
        true
    }

    fn collect_access(_reads: &mut AccessVec<TypeId>, writes: &mut AccessVec<TypeId>) {
        writes.push(TypeId::of::<T>());
        writes.push(TypeId::of::<ChangedMarker<T>>());
    }

    unsafe fn init_fetch(archetype: &Archetype, _: &mut FunctionData) -> Self::Fetch {
        unsafe {
            let columns = &mut *archetype.columns.get();
            let component_id = TypeId::of::<T>();
            let marker_id = TypeId::of::<ChangedMarker<T>>();

            let current_generation = CURRENT_FRAME_GENERATION.load(Ordering::Relaxed);

            if columns.contains_key(&component_id) {
                let data_ptr = (*archetype.fetch_column_raw::<T>()).as_mut_ptr();

                if let Some(column) = columns.get_mut(&marker_id) {
                    let vec_ptr = column
                        .data
                        .as_any_mut()
                        .downcast_mut::<Vec<ChangedMarker<T>>>()
                        .unwrap();

                    (
                        Some(data_ptr),
                        vec_ptr.as_mut_ptr(),
                        current_generation,
                        changed_marker_modify,
                    )
                } else {
                    (
                        Some(data_ptr),
                        std::ptr::null_mut(),
                        current_generation,
                        no_op_mut,
                    )
                }
            } else {
                (None, std::ptr::null_mut(), current_generation, no_op_mut)
            }
        }
    }

    #[inline(always)]
    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
        unsafe {
            if let Some(data_head) = fetch.0 {
                Some(Mut {
                    value: data_head.add(index),
                    marker: fetch.1.add(index),
                    deref_mut_function: fetch.3,
                    generation: fetch.2,
                    _marker: std::marker::PhantomData,
                })
            } else {
                None
            }
        }
    }

    #[inline(always)]
    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
        unsafe {
            if let Some(data_head) = fetch.0 {
                Some(&*data_head.add(index))
            } else {
                None
            }
        }
    }
}

macro_rules! impl_world_query_tuple {
    ($($name:ident -> $idx:tt),*) => {
        impl<$($name: WorldQuery),*> WorldQuery for ($($name,)*) {
            type Item<'w> = ($($name::Item<'w>,)*);
            type ReadOnlyItem<'w> = ($($name::ReadOnlyItem<'w>,)*);
            type Fetch = ($($name::Fetch,)*);

            fn matches(types: &HashSet<TypeId>) -> bool { $($name::matches(types))&&* }
            unsafe fn init_fetch(archetype: &Archetype, systems_data: &mut FunctionData) -> Self::Fetch { unsafe { ($($name::init_fetch(archetype, systems_data),)*) } }
            unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w> {
                unsafe {($($name::fetch_mut(fetch.$idx, index),)*)}
            }

            unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w> {
                unsafe { ($($name::fetch_read_only(fetch.$idx, index),)*) }
            }

            fn collect_access(reads: &mut AccessVec<TypeId>, writes: &mut AccessVec<TypeId>) {
                $( $name::collect_access(reads, writes); )*
            }
        }
    };
}

impl_world_query_tuple!(A -> 0);
impl_world_query_tuple!(A -> 0, B -> 1);
impl_world_query_tuple!(A -> 0, B -> 1, C -> 2);
impl_world_query_tuple!(A -> 0, B -> 1, C -> 2, D -> 3);
impl_world_query_tuple!(A -> 0, B -> 1, C -> 2, D -> 3, E -> 4);
impl_world_query_tuple!(A -> 0, B -> 1, C -> 2, D -> 3, E -> 4, F -> 5);
impl_world_query_tuple!(A -> 0, B -> 1, C -> 2, D -> 3, E -> 4, F -> 5, G -> 6);
impl_world_query_tuple!(A -> 0, B -> 1, C -> 2, D -> 3, E -> 4, F -> 5, G -> 6, H -> 7);
impl_world_query_tuple!(A -> 0, B -> 1, C -> 2, D -> 3, E -> 4, F -> 5, G -> 6, H -> 7, I -> 8);
impl_world_query_tuple!(A -> 0, B -> 1, C -> 2, D -> 3, E -> 4, F -> 5, G -> 6, H -> 7, I -> 8, J -> 9);
impl_world_query_tuple!(A -> 0, B -> 1, C -> 2, D -> 3, E -> 4, F -> 5, G -> 6, H -> 7, I -> 8, J -> 9, K -> 10);
