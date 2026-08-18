use std::{any::TypeId, collections::HashSet};

use crate::{
    query::changed::{Changed, ChangedMarker},
    world::archetypes::Archetype,
};

pub trait Filter {
    fn matches(types: &HashSet<TypeId>) -> bool;
    fn collect_filter(withs: &mut Vec<std::any::TypeId>, withouts: &mut Vec<std::any::TypeId>);

    #[inline(always)]
    fn collect_tracking(_tracked: &mut Vec<std::any::TypeId>) {}

    #[inline(always)]
    unsafe fn filter_indices(_archetype: &Archetype, _indices: &mut Vec<usize>) {}
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

    unsafe fn filter_indices(archetype: &Archetype, indices: &mut Vec<usize>) {
        let marker_ptr = unsafe { (*archetype.fetch_column_raw::<ChangedMarker<T>>()).as_ptr() };
        indices.retain(|&idx| unsafe { (*marker_ptr.add(idx)).0 > 0 });
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
            unsafe fn filter_indices(archetype: &Archetype, indices: &mut Vec<usize>) {
                $(
                    unsafe { $name::filter_indices(archetype, indices); }
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
