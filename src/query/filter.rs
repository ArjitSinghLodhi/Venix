use std::any::TypeId;

use crate::{
    query::ergonomic_params::{AnyOf, Not, Or},
    system::validation::{AccessHashSet, AccessVec, FunctionData},
    world::archetypes::Archetype,
};

pub trait StructuralQueryFilter: QueryFilter {}

impl<T: 'static> StructuralQueryFilter for With<T> {}
impl<T: 'static> StructuralQueryFilter for Without<T> {}
impl<A: QueryFilter + StructuralQueryFilter, B: QueryFilter + StructuralQueryFilter>
    StructuralQueryFilter for Or<A, B>
{
}
impl<T: QueryFilter + StructuralQueryFilter> StructuralQueryFilter for Not<T> {}
impl<T: QueryFilter> StructuralQueryFilter for AnyOf<T> where AnyOf<T>: QueryFilter {}

pub trait QueryFilter {
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
impl<T: 'static> QueryFilter for With<T> {
    fn matches(types: &AccessHashSet<TypeId>) -> bool {
        types.contains(&TypeId::of::<T>())
    }
    fn collect_filter(withs: &mut AccessVec<TypeId>, _withouts: &mut AccessVec<TypeId>) {
        withs.push(std::any::TypeId::of::<T>());
    }
}
pub struct Without<T>(std::marker::PhantomData<T>);
impl<T: 'static> QueryFilter for Without<T> {
    fn matches(types: &AccessHashSet<TypeId>) -> bool {
        !types.contains(&TypeId::of::<T>())
    }
    fn collect_filter(_withs: &mut AccessVec<TypeId>, withouts: &mut AccessVec<TypeId>) {
        withouts.push(std::any::TypeId::of::<T>());
    }
}
pub struct EmptyQueryFilter;
impl QueryFilter for EmptyQueryFilter {
    fn matches(_: &AccessHashSet<TypeId>) -> bool {
        true
    }
    fn collect_filter(_withs: &mut AccessVec<TypeId>, _withouts: &mut AccessVec<TypeId>) {}
}

macro_rules! impl_QueryFilter_tuple {
    ($($name:ident),*) => {
        impl<$($name: QueryFilter),*> QueryFilter for ($($name,)*) {
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
        impl<$($name: QueryFilter + StructuralQueryFilter),*> StructuralQueryFilter for ($($name,)*){}
    };
}

impl_QueryFilter_tuple!(A);
impl_QueryFilter_tuple!(A, B);
impl_QueryFilter_tuple!(A, B, C);
impl_QueryFilter_tuple!(A, B, C, D);
impl_QueryFilter_tuple!(A, B, C, D, E);
impl_QueryFilter_tuple!(A, B, C, D, E, F);
impl_QueryFilter_tuple!(A, B, C, D, E, F, G);
impl_QueryFilter_tuple!(A, B, C, D, E, F, G, H);
impl_QueryFilter_tuple!(A, B, C, D, E, F, G, H, I);
impl_QueryFilter_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_QueryFilter_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_QueryFilter_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_QueryFilter_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
