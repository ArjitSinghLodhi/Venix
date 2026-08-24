use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use std::any::TypeId;

use crate::world::archetypes::{Archetype, ComponentColumn};

pub trait Bundle: 'static {
    const TYPE_IDS: &[TypeId];
    fn get_type_ids() -> &'static [TypeId];
    fn push_to_archetype(self, archetype: &mut Archetype);
    fn create_empty_columns(columns: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>);

    type NamesArray: AsRef<[&'static str]>;
    fn get_type_names() -> Self::NamesArray;
}

macro_rules! impl_component_tuple {
    ($($T:ident),*) => {
        impl<$($T: 'static),*> Bundle for ($($T,)*) {

            const TYPE_IDS: &[TypeId] = &[ $( TypeId::of::<$T>() ),* ];

            fn get_type_ids() -> &'static [TypeId] {
                Self::TYPE_IDS
            }

            fn create_empty_columns(columns: &mut IndexMap<TypeId, ComponentColumn, FxBuildHasher>) {
                $(
                    let id = TypeId::of::<$T>();
                    columns.insert(id, ComponentColumn {
                        data: Box::new(Vec::<$T>::new()),
                    });
                )*
            }

            fn push_to_archetype(self, archetype: &mut Archetype) {
                #[allow(non_snake_case)]
                let ($($T,)*) = self;
                unsafe {
                    $(
                        let vec_ptr = archetype.fetch_column_raw::<$T>();
                        if !vec_ptr.is_null() {
                            (*vec_ptr).push($T);
                        }
                    )*
                }
            }

            type NamesArray = [&'static str; 0 $( + { let _ = stringify!($T); 1 } )*];

            #[inline(always)]
            fn get_type_names() -> Self::NamesArray {
                [ $( std::any::type_name::<$T>() ),* ]
            }
        }
    };
}

impl_component_tuple!(A);
impl_component_tuple!(A, B);
impl_component_tuple!(A, B, C);
impl_component_tuple!(A, B, C, D);
impl_component_tuple!(A, B, C, D, E);
impl_component_tuple!(A, B, C, D, E, F);
impl_component_tuple!(A, B, C, D, E, F, G);
impl_component_tuple!(A, B, C, D, E, F, G, H);
impl_component_tuple!(A, B, C, D, E, F, G, H, I);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_component_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
