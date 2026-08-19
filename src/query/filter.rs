use std::{any::TypeId, collections::HashSet};

use crate::{
    query::ergonomic_params::{AnyOf, Or},
    system::validation::FunctionData,
    world::archetypes::Archetype,
};

pub trait StructuralFilter: Filter {}

impl<T: Filter + 'static> StructuralFilter for With<T> {}
impl<T: Filter + 'static> StructuralFilter for Without<T> {}
impl<A: Filter + StructuralFilter, B: Filter + StructuralFilter> StructuralFilter for Or<A, B> {}
impl<T: Filter> StructuralFilter for AnyOf<T> where AnyOf<T>: Filter {}

pub trait Filter {
    fn matches(types: &HashSet<TypeId>) -> bool;
    fn collect_filter(withs: &mut Vec<std::any::TypeId>, withouts: &mut Vec<std::any::TypeId>);
    fn filter_indices(
        _archetype: &Archetype,
        _indices: &mut Vec<usize>,
        _system_data: &mut FunctionData,
    ) {
    }
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

            #[inline]
            fn filter_indices(archetype: &Archetype, indices: &mut Vec<usize>, systems_data: &mut FunctionData) {
                $(
                    $name::filter_indices(archetype, indices, systems_data);
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
