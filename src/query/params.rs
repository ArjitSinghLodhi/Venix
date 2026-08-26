use std::any::TypeId;

use fxhash::FxHashSet;

use crate::{
    entity::Entity,
    query::{
        changed::{ChangedMarker, Mut},
        query::QueryData,
    },
    system::validation::{AccessVec, FunctionData, FunctionGenerationData},
    world::{archetypes::Archetype, storage::GenerationRing},
};

impl<T: 'static> QueryData for &T {
    type Item<'w> = &'w T;
    type ReadOnlyItem<'w> = &'w T;
    type Fetch = *const T;

    fn matches(types: &FxHashSet<TypeId>) -> bool {
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

impl<T: 'static> QueryData for &mut T {
    type Item<'w> = Mut<'w, T>;
    type ReadOnlyItem<'w> = &'w T;
    type Fetch = (*mut T, *mut ChangedMarker<T>, u8, u32, bool);

    fn matches(types: &FxHashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<T>())
    }

    unsafe fn init_fetch(archetype: &Archetype, data: &mut FunctionData) -> Self::Fetch {
        unsafe {
            let data_ptr = (*archetype.fetch_column_raw::<T>()).as_mut_ptr();
            let columns = &mut *archetype.columns.get();
            let marker_id = TypeId::of::<ChangedMarker<T>>();
            let generation_data = data
                .get_data::<FunctionGenerationData>()
                .expect("Missing FunctionGenerationData on mutable component query fetch");

            let system_id = generation_data.system_id;
            let current_generation = GenerationRing::current();

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
                    system_id,
                    true,
                )
            } else {
                (
                    data_ptr,
                    std::ptr::null_mut(),
                    current_generation,
                    system_id,
                    false,
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
                generation: fetch.2,
                system_id: fetch.3,
                should_modify: fetch.4,
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

impl QueryData for Entity {
    type Item<'w> = &'w Entity;
    type ReadOnlyItem<'w> = &'w Entity;
    type Fetch = *const Entity;

    fn matches(_types: &FxHashSet<TypeId>) -> bool {
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

impl<T: 'static> QueryData for Option<&T> {
    type Item<'w> = Option<&'w T>;
    type ReadOnlyItem<'w> = Option<&'w T>;
    type Fetch = Option<*const T>;

    fn matches(_types: &FxHashSet<TypeId>) -> bool {
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

impl<T: 'static> QueryData for Option<&mut T> {
    type Item<'w> = Option<Mut<'w, T>>;
    type ReadOnlyItem<'w> = Option<&'w T>;
    type Fetch = (Option<*mut T>, *mut ChangedMarker<T>, u8, u32, bool);

    fn matches(_types: &FxHashSet<TypeId>) -> bool {
        true
    }

    fn collect_access(_reads: &mut AccessVec<TypeId>, writes: &mut AccessVec<TypeId>) {
        writes.push(TypeId::of::<T>());
        writes.push(TypeId::of::<ChangedMarker<T>>());
    }

    unsafe fn init_fetch(archetype: &Archetype, data: &mut FunctionData) -> Self::Fetch {
        unsafe {
            let columns = &mut *archetype.columns.get();
            let component_id = TypeId::of::<T>();
            let marker_id = TypeId::of::<ChangedMarker<T>>();
            let generation_data = data
                .get_data::<FunctionGenerationData>()
                .expect("Missing FunctionGenerationData on optional mutable component query fetch");

            let system_id = generation_data.system_id;
            let current_generation = GenerationRing::current();

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
                        system_id,
                        true,
                    )
                } else {
                    (
                        Some(data_ptr),
                        std::ptr::null_mut(),
                        current_generation,
                        system_id,
                        false,
                    )
                }
            } else {
                (
                    None,
                    std::ptr::null_mut(),
                    current_generation,
                    system_id,
                    false,
                )
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
                    generation: fetch.2,
                    system_id: fetch.3,
                    should_modify: fetch.4,
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
        impl<$($name: QueryData),*> QueryData for ($($name,)*) {
            type Item<'w> = ($($name::Item<'w>,)*);
            type ReadOnlyItem<'w> = ($($name::ReadOnlyItem<'w>,)*);
            type Fetch = ($($name::Fetch,)*);

            fn matches(types: &FxHashSet<TypeId>) -> bool { $($name::matches(types))&&* }
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
