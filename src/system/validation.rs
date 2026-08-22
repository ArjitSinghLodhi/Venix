use std::{any::TypeId, collections::HashSet, hash::Hash};

use crate::{
    query::{filter::Filter, params::WorldQuery, query::Query},
    resources::{Res, ResMut},
    world::storage::World,
};

pub struct AccessHashSet<T: Eq + Hash> {
    pub(crate) set: HashSet<T>,
}

impl<T: Eq + Hash> AccessHashSet<T> {
    pub(crate) fn new() -> Self {
        Self {
            set: HashSet::new(),
        }
    }

    pub fn insert(&mut self, val: T) -> bool {
        self.set.insert(val)
    }

    pub fn contains(&self, val: &T) -> bool {
        self.set.contains(val)
    }
}

impl<T: Eq + Hash> Default for AccessHashSet<T> {
    fn default() -> Self {
        AccessHashSet::new()
    }
}

pub struct AccessVec<T> {
    pub(crate) vec: Vec<T>,
}

impl<T> Default for AccessVec<T> {
    fn default() -> Self {
        Self { vec: Vec::new() }
    }
}

impl<T> AccessVec<T> {
    pub fn push(&mut self, val: T) {
        self.vec.push(val);
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.vec.iter()
    }
}

#[derive(Default)]
pub struct ParamAccess {
    pub reads: AccessVec<TypeId>,
    pub writes: AccessVec<std::any::TypeId>,
    pub with_filters: AccessVec<std::any::TypeId>,
    pub without_filters: AccessVec<std::any::TypeId>,
    pub res_reads: AccessVec<std::any::TypeId>,
    pub res_writes: AccessVec<std::any::TypeId>,
}

pub trait SystemParam {
    fn get_access() -> ParamAccess;
    fn extract(world: &mut World, system_data: &mut FunctionData) -> Self;
}

impl<Q: WorldQuery + 'static, F: Filter + 'static> SystemParam for Query<Q, F> {
    fn get_access() -> ParamAccess {
        let mut access = ParamAccess::default();
        Q::collect_access(&mut access.reads, &mut access.writes);
        F::collect_filter(&mut access.with_filters, &mut access.without_filters);
        access
    }

    fn extract(world: &mut World, system_data: &mut FunctionData) -> Self {
        Query::<Q, F>::new(world, system_data)
    }
}

pub trait System {
    fn run(&mut self, world: &mut World);
}

pub struct FunctionSystem<Marker, F> {
    pub func: F,
    pub data: FunctionData,
    pub _marker: std::marker::PhantomData<Marker>,
}

impl<Marker, F> FunctionSystem<Marker, F> {
    pub fn new(func: F) -> Self {
        Self {
            func,
            data: FunctionData::new(),
            _marker: std::marker::PhantomData,
        }
    }
}

pub struct FunctionData {
    pub(crate) last_run_generation: u8,
    pub(crate) current_run_generation: u8,
}

impl FunctionData {
    pub fn new() -> Self {
        Self {
            last_run_generation: 0,
            current_run_generation: 0,
        }
    }
}

pub trait IntoSystem<Marker> {
    type SystemType: System + 'static;
    fn into_system(self) -> Self::SystemType;
}
pub trait IntoSystemConfigs<MarkerGroup> {
    fn add_to_schedule(self, schedule: &mut Vec<Box<dyn System>>);
}

macro_rules! impl_system_configs_tuple {
    ($($sys:ident),* ; $($marker:ident),*) => {
        impl<$($sys,)* $($marker,)*> IntoSystemConfigs<($($marker,)*)> for ($($sys,)*)
        where
            $( $sys: IntoSystem<$marker> + 'static ),*
        {
            fn add_to_schedule(self, schedule: &mut Vec<Box<dyn System>>) {
                #[allow(non_snake_case)]
                let ($($sys,)*) = self;
                $(
                    schedule.push(Box::new($sys.into_system()));
                )*
            }
        }
    };
}

impl_system_configs_tuple!(S1, S2 ; M1, M2);
impl_system_configs_tuple!(S1, S2, S3 ; M1, M2, M3);
impl_system_configs_tuple!(S1, S2, S3, S4; M1, M2, M3, M4);
impl_system_configs_tuple!(S1, S2, S3, S4, S5; M1, M2, M3, M4, M5);
impl_system_configs_tuple!(S1, S2, S3, S4, S5, S6; M1, M2, M3, M4, M5, M6);
impl_system_configs_tuple!(S1, S2, S3, S4, S5, S6, S7; M1, M2, M3, M4, M5, M6, M7);
impl_system_configs_tuple!(S1, S2, S3, S4, S5, S6, S7, S8; M1, M2, M3, M4, M5, M6, M7, M8);
impl<S, Marker> IntoSystemConfigs<(Marker,)> for S
where
    S: IntoSystem<Marker> + 'static,
{
    fn add_to_schedule(self, schedule: &mut Vec<Box<dyn System>>) {
        schedule.push(Box::new(self.into_system()));
    }
}

impl<T: 'static> SystemParam for Res<T> {
    fn get_access() -> ParamAccess {
        let mut access = ParamAccess {
            ..Default::default()
        };
        access.res_reads.push(std::any::TypeId::of::<T>());
        access
    }

    fn extract(world: &mut World, _system_data: &mut FunctionData) -> Self {
        unsafe { Self::new(world) }
    }
}

impl<T: 'static> SystemParam for ResMut<T> {
    fn get_access() -> ParamAccess {
        let mut access = ParamAccess {
            ..Default::default()
        };
        access.res_writes.push(std::any::TypeId::of::<T>());
        access
    }

    fn extract(world: &mut World, _system_data: &mut FunctionData) -> Self {
        unsafe { Self::new(world) }
    }
}
