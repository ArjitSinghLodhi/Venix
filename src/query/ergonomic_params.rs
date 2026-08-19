use crate::query::filter::Filter;
use crate::system::validation::FunctionData;
use crate::world::archetypes::Archetype;
use std::{any::TypeId, collections::HashSet};

pub struct Or<A, B>(std::marker::PhantomData<(A, B)>);

pub struct AnyOf<T>(std::marker::PhantomData<T>);

pub struct Not<F>(std::marker::PhantomData<F>);

impl<F: Filter> Filter for Not<F> {
    #[inline]
    fn matches(types: &HashSet<TypeId>) -> bool {
        !F::matches(types)
    }

    #[inline]
    fn collect_filter(withs: &mut Vec<TypeId>, withouts: &mut Vec<TypeId>) {
        F::collect_filter(withs, withouts);
    }

    #[inline(always)]
    fn collect_tracking(tracked: &mut Vec<std::any::TypeId>) {
        F::collect_tracking(tracked);
    }

    #[inline(always)]
    fn filter_indices(
        archetype: &crate::world::archetypes::Archetype,
        indices: &mut Vec<usize>,
        system_data: &mut crate::system::validation::FunctionData,
    ) {
        F::filter_indices(archetype, indices, system_data);
    }
}

impl<A: Filter, B: Filter> Filter for Or<A, B> {
    #[inline]
    fn matches(types: &HashSet<TypeId>) -> bool {
        A::matches(types) || B::matches(types)
    }

    #[inline]
    fn collect_filter(withs: &mut Vec<TypeId>, withouts: &mut Vec<TypeId>) {
        A::collect_filter(withs, withouts);
        B::collect_filter(withs, withouts);
    }

    #[inline(always)]
    fn collect_tracking(tracked: &mut Vec<std::any::TypeId>) {
        A::collect_tracking(tracked);
        B::collect_tracking(tracked);
    }

    #[inline(always)]
    fn filter_indices(
        archetype: &crate::world::archetypes::Archetype,
        indices: &mut Vec<usize>,
        system_data: &mut crate::system::validation::FunctionData,
    ) {
        A::filter_indices(archetype, indices, system_data);
        B::filter_indices(archetype, indices, system_data);
    }
}

macro_rules! impl_any_of_tuple {
    ($($name:ident),*) => {
        impl<$($name: Filter),*> Filter for AnyOf<($($name,)*)> {
            #[inline]
            fn matches(types: &HashSet<TypeId>) -> bool {
                $($name::matches(types))||*
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
            fn filter_indices(archetype: &Archetype, indices: &mut Vec<usize>, systems_data: &mut FunctionData) {
                $(
                    $name::filter_indices(archetype, indices, systems_data);
                )*
            }
        }
    };
}

impl_any_of_tuple!(A);
impl_any_of_tuple!(A, B);
impl_any_of_tuple!(A, B, C);
impl_any_of_tuple!(A, B, C, D);
impl_any_of_tuple!(A, B, C, D, E);
impl_any_of_tuple!(A, B, C, D, E, F);
