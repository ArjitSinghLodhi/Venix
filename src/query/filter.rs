use std::{any::TypeId, collections::HashSet};

use crate::{
    query::changed::{Changed, ChangedMarker}, system::validation::FunctionData, world::archetypes::Archetype
};

pub trait Filter {
    fn matches(types: &HashSet<TypeId>) -> bool;
    fn collect_filter(withs: &mut Vec<std::any::TypeId>, withouts: &mut Vec<std::any::TypeId>);

    #[inline(always)]
    fn collect_tracking(_tracked: &mut Vec<std::any::TypeId>) {}
    unsafe fn filter_indices(
        _archetype: &Archetype, 
        _indices: &mut Vec<usize>, 
        _system_data: &mut FunctionData
    ) {}
}

pub struct With<T>(std::marker::PhantomData<T>);
impl<T: 'static> Filter for With<T> {
    fn matches(types: &HashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<T>())
    }
    fn collect_filter(withs: &mut Vec<TypeId>, _withouts: &mut Vec<TypeId>) {
        withs.push(std::any::TypeId::of::<T>());
    }
}
pub struct Without<T>(std::marker::PhantomData<T>);
impl<T: 'static> Filter for Without<T> {
    fn matches(types: &HashSet<TypeId>) -> bool {
        !types.contains(&TypeId::of::<T>())
    }
    fn collect_filter(_withs: &mut Vec<TypeId>, withouts: &mut Vec<TypeId>) {
        withouts.push(std::any::TypeId::of::<T>());
    }
}
pub struct EmptyFilter;
impl Filter for EmptyFilter {
    fn matches(_: &HashSet<TypeId>) -> bool {
        true
    }
    fn collect_filter(_withs: &mut Vec<TypeId>, _withouts: &mut Vec<TypeId>) {}
}

impl<T: 'static + Send + Sync> Filter for Changed<T> {
    fn matches(types: &HashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<T>()) && types.contains(&TypeId::of::<ChangedMarker<T>>())
    }

    fn collect_filter(withs: &mut Vec<TypeId>, _withouts: &mut Vec<TypeId>) {
        withs.push(TypeId::of::<ChangedMarker<T>>());
    }

    #[inline(always)]
    fn collect_tracking(tracked: &mut Vec<std::any::TypeId>) {
        tracked.push(std::any::TypeId::of::<T>());
        crate::query::changed::register_tracked_component::<T>();
    }

    unsafe fn filter_indices(
        archetype: &Archetype, 
        indices: &mut Vec<usize>, 
        system_data: &mut FunctionData
    ) {
        let marker_ptr = unsafe { (*archetype.fetch_column_raw::<ChangedMarker<T>>()).as_ptr() };
        let current_generation = system_data.current_run_generation;
        let system_last_generation = system_data.last_run_generation;
        let previous_generation = if current_generation == 1 { 2 } else { 1 };

        indices.retain(|&idx| unsafe {
            let marker_val = (*marker_ptr.add(idx)).0;

            if marker_val == 0 { return false; }

            if marker_val == current_generation {
                return system_last_generation != current_generation;
            }
            if marker_val == previous_generation {
                return system_last_generation != previous_generation && system_last_generation != current_generation;
            }

            false
        });
    }
}


macro_rules! impl_filter_tuple {
    ($($name:ident),*) => {
        impl<$($name: Filter),*> Filter for ($($name,)*) {
            #[inline]
            fn matches(types: &HashSet<TypeId>) -> bool {
                $($name::matches(types))&&*
            }

            #[inline]
            fn collect_filter(withs: &mut Vec<TypeId>, withouts: &mut Vec<TypeId>) {
                $(
                    $name::collect_filter(withs, withouts);
                )*
            }
            fn collect_tracking(tracked: &mut Vec<TypeId>) {
                $(
                    $name::collect_tracking(tracked);
                )*
            }

            #[inline]
            unsafe fn filter_indices(archetype: &Archetype, indices: &mut Vec<usize>, systems_data: &mut FunctionData) {
                $(
                    unsafe { $name::filter_indices(archetype, indices, systems_data); }
                )*
            }
        }
    };
}

impl_filter_tuple!(A);
impl_filter_tuple!(A, B);
impl_filter_tuple!(A, B, C);
impl_filter_tuple!(A, B, C, D);
impl_filter_tuple!(A, B, C, D, E);
impl_filter_tuple!(A, B, C, D, E, F);
impl_filter_tuple!(A, B, C, D, E, F, G);
impl_filter_tuple!(A, B, C, D, E, F, G, H);
