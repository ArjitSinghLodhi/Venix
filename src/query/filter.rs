use std::any::TypeId;

use crate::{
    query::ergonomic_params::{AnyOf, Not, Or},
    system::validation::{AccessHashSet, AccessVec, FunctionData},
    world::archetypes::Archetype,
};

pub trait StructuralFilter: Filter {}

impl<T: 'static> StructuralFilter for With<T> {}
impl<T: 'static> StructuralFilter for Without<T> {}
impl<A: Filter + StructuralFilter, B: Filter + StructuralFilter> StructuralFilter for Or<A, B> {}
impl<T: Filter + StructuralFilter> StructuralFilter for Not<T> {}
impl<T: Filter> StructuralFilter for AnyOf<T> where AnyOf<T>: Filter {}

pub trait Filter {
    fn matches(types: &AccessHashSet<TypeId>) -> bool;
    fn matches_negated(types: &AccessHashSet<TypeId>) -> bool {
        !Self::matches(types)
    }
    fn collect_filter(
        withs: &mut AccessVec<std::any::TypeId>,
        withouts: &mut AccessVec<std::any::TypeId>,
    );
    fn filter_indices(
        _archetype: &Archetype,
        _indices: &mut Vec<usize>,
        _system_data: &mut FunctionData,
    ) {
    }
}

pub struct With<T>(std::marker::PhantomData<T>);
impl<T: 'static> Filter for With<T> {
    fn matches(types: &AccessHashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<T>())
    }
    fn collect_filter(withs: &mut AccessVec<TypeId>, _withouts: &mut AccessVec<TypeId>) {
        withs.push(std::any::TypeId::of::<T>());
    }
}
pub struct Without<T>(std::marker::PhantomData<T>);
impl<T: 'static> Filter for Without<T> {
    fn matches(types: &AccessHashSet<TypeId>) -> bool {
        !types.contains(&TypeId::of::<T>())
    }
    fn collect_filter(_withs: &mut AccessVec<TypeId>, withouts: &mut AccessVec<TypeId>) {
        withouts.push(std::any::TypeId::of::<T>());
    }
}
pub struct EmptyFilter;
impl Filter for EmptyFilter {
    fn matches(_: &AccessHashSet<TypeId>) -> bool {
        true
    }
    fn collect_filter(_withs: &mut AccessVec<TypeId>, _withouts: &mut AccessVec<TypeId>) {}
}

macro_rules! impl_filter_tuple {
    ($($name:ident),*) => {
        impl<$($name: Filter),*> Filter for ($($name,)*) {
            #[inline]
            fn matches(types: &AccessHashSet<TypeId>) -> bool {
                $($name::matches(types))&&*
            }

            fn matches_negated(types: &AccessHashSet<TypeId>) -> bool {
                $($name::matches_negated(types))&&*
            }

            #[inline]
            fn collect_filter(withs: &mut AccessVec<TypeId>, withouts: &mut AccessVec<TypeId>) {
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
        impl<$($name: Filter + StructuralFilter),*> StructuralFilter for ($($name,)*){}
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
impl_filter_tuple!(A, B, C, D, E, F, G, H, I);
impl_filter_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_filter_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_filter_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_filter_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
