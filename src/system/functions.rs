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
            fn run(&self, world: &mut World) {
                $(
                    let $var = <$param>::extract(world);
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
                    let has_intra_read_write_conflict = param.writes.iter().any(|w| param.reads.contains(w));

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

                        let has_write_conflict = param_a.writes.iter().any(|w| param_b.writes.contains(w) || param_b.reads.contains(w))
                            || param_b.writes.iter().any(|w| param_a.writes.contains(w) || param_a.reads.contains(w));

                        if has_write_conflict {
                            let is_disjoint = param_a.with_filters.iter().any(|f| param_b.without_filters.contains(f))
                                || param_b.with_filters.iter().any(|f| param_a.without_filters.contains(f));

                            if !is_disjoint {
                                panic!(
                                    "❌ ECS SYSTEM ARGUMENT CONFLICT: Function '{}' contains overlapping queries that mutably borrow the same component data without disjoint filters (With/Without)!",
                                    std::any::type_name::<F>()
                                );
                            }
                        }

                        let has_resource_conflict = param_a.res_writes.iter().any(|rw| param_b.res_writes.contains(rw) || param_b.res_reads.contains(rw))
                            || param_b.res_writes.iter().any(|rw| param_a.res_writes.contains(rw) || param_a.res_reads.contains(rw));

                        if has_resource_conflict {
                            panic!(
                                "❌ ECS RESOURCE BORROW CONFLICT: Function '{}' contains conflicting parameters (e.g. ResMut alongside Res, or duplicate ResMut) targeting the same global resource singleton!",
                                std::any::type_name::<F>()
                            );
                        }
                        let has_commands_conflict = param_a.commands_accessed.iter().any(|rw| param_b.commands_accessed.contains(rw))
                            || param_b.commands_accessed.iter().any(|rw| param_a.commands_accessed.contains(rw));
                        if has_commands_conflict {
                            panic!(
                                "❌ ECS Commands Argument CONFLICT: Function '{}' contains conflicting parameters (Duplicate Commands parameter) targeting the same Commands Buffer!",
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
