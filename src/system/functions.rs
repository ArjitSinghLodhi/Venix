use crate::system::validation::FunctionSystem;
use crate::system::validation::IntoSystem;
use crate::system::validation::System;
use crate::system::validation::SystemParam;
use crate::world::storage::World;
macro_rules! impl_system_for_functions {
    ($($var:ident : $param:ident),*) => {
        impl<$($param,)* F> System for FunctionSystem<($($param,)*), F>
        where
            $( $param: SystemParam + 'static, )*
            F: Fn($($param),*) + 'static,
        {
            fn run(&mut self, world: &mut World) {
                let data = &mut self.data;
                $(
                    let $var = <$param>::extract(world, data);
                )*
                (self.func)($($var),*);
            }
        }

        impl<$($param,)* F> IntoSystem<($($param,)*)> for F
        where
            $( $param: SystemParam + 'static, )*
            F: Fn($($param),*) + 'static,
        {
            type SystemType = FunctionSystem<($($param,)*), F>;
            fn into_system(self) -> Self::SystemType {
                let mut params_access = Vec::new();
                $(
                    params_access.push(<$param>::get_access());
                )*

                for param in &params_access {
                    let has_intra_read_write_conflict = param.writes.iter().any(|w| param.reads.vec.contains(w));

                    let mut unique_writes = std::collections::HashSet::new();
                    let has_duplicate_writes = param.writes.iter().any(|w| !unique_writes.insert(w));

                    if has_intra_read_write_conflict || has_duplicate_writes {
                        panic!(
                            "❌ ECS INTRA-QUERY ARGUMENT CONFLICT: Function '{}' contains internal query overlaps (e.g., duplicated mutable borrows) within a single parameter!",
                            std::any::type_name::<F>()
                        );
                    }
                }
                for i in 0..params_access.len() {
                    for j in 0..params_access.len() {
                        if i == j { continue; }
                        let param_a = &params_access[i];
                        let param_b = &params_access[j];

                        let has_write_conflict = param_a.writes.iter().any(|w| param_b.writes.vec.contains(w) || param_b.reads.vec.contains(w))
                            || param_b.writes.iter().any(|w| param_a.writes.vec.contains(w) || param_a.reads.vec.contains(w));

                        if has_write_conflict {
                            let is_disjoint = param_a.with_filters.iter().any(|f| param_b.without_filters.vec.contains(f))
                                || param_b.with_filters.iter().any(|f| param_a.without_filters.vec.contains(f));

                            if !is_disjoint {
                                panic!(
                                    "❌ ECS SYSTEM ARGUMENT CONFLICT: Function '{}' contains overlapping queries that mutably borrow the same component data without disjoint filters (With/Without)!",
                                    std::any::type_name::<F>()
                                );
                            }
                        }

                        let has_resource_conflict = param_a.res_writes.iter().any(|rw| param_b.res_writes.vec.contains(rw) || param_b.res_reads.vec.contains(rw))
                            || param_b.res_writes.iter().any(|rw| param_a.res_writes.vec.contains(rw) || param_a.res_reads.vec.contains(rw));

                        if has_resource_conflict {
                            panic!(
                                "❌ ECS RESOURCE BORROW CONFLICT: Function '{}' contains conflicting parameters (e.g. ResMut alongside Res, or duplicate ResMut) targeting the same global resource singleton!",
                                std::any::type_name::<F>()
                            );
                        }
                    }
                }

                FunctionSystem::new(self)
            }
        }
    };
}

impl_system_for_functions!(a: A);
impl_system_for_functions!(a: A, b: B);
impl_system_for_functions!(a: A, b: B, c: C);
impl_system_for_functions!(a: A, b: B, c: C, d: D);
impl_system_for_functions!(a: A, b: B, c: C, d: D, e: E);
impl_system_for_functions!(a: A, b: B, c: C, d: D, e: E, g: G);
impl_system_for_functions!(a: A, b: B, c: C, d: D, e: E, g: G, h: H);
impl_system_for_functions!(a: A, b: B, c: C, d: D, e: E, g: G, h: H, i: I);
