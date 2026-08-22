use crate::{
    query::filter::{Filter, StructuralFilter},
    system::validation::{AccessHashSet, AccessVec},
};
use std::any::TypeId;

pub struct Or<A, B>(std::marker::PhantomData<(A, B)>);

pub struct AnyOf<T>(std::marker::PhantomData<T>);

pub struct Not<F>(std::marker::PhantomData<F>);

impl<F: Filter + StructuralFilter> Filter for Not<F> {
    #[inline]
    fn matches(types: &AccessHashSet<TypeId>) -> bool {
        F::matches_negated(types)
    }

    #[inline]
    fn collect_filter(withs: &mut AccessVec<TypeId>, withouts: &mut AccessVec<TypeId>) {
        F::collect_filter(withouts, withs);
    }
}

impl<A: Filter + StructuralFilter, B: Filter + StructuralFilter> Filter for Or<A, B> {
    #[inline]
    fn matches(types: &AccessHashSet<TypeId>) -> bool {
        A::matches(types) || B::matches(types)
    }

    #[inline]
    fn collect_filter(withs: &mut AccessVec<TypeId>, withouts: &mut AccessVec<TypeId>) {
        A::collect_filter(withs, withouts);
        B::collect_filter(withs, withouts);
    }
}

macro_rules! impl_any_of_tuple {
    ($($name:ident),*) => {
        impl<$($name: Filter + StructuralFilter),*> Filter for AnyOf<($($name,)*)> {
            #[inline]
            fn matches(types: &AccessHashSet<TypeId>) -> bool {
                $($name::matches(types))||*
            }
            #[inline]
            fn collect_filter(withs: &mut AccessVec<TypeId>, withouts: &mut AccessVec<TypeId>) {
                $(
                    $name::collect_filter(withs, withouts);
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
impl_any_of_tuple!(A, B, C, D, E, F, G);
impl_any_of_tuple!(A, B, C, D, E, F, G, H);
impl_any_of_tuple!(A, B, C, D, E, F, G, H, I);
impl_any_of_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_any_of_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_any_of_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
