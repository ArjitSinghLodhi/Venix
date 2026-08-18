use crate::{
    query::{
        filter::Filter,
        params::WorldQuery,
        query::Query,
        resources::{Res, ResMut},
    },
    world::storage::World,
};

#[derive(Default)]
pub struct ParamAccess {
    pub reads: Vec<std::any::TypeId>,
    pub writes: Vec<std::any::TypeId>,
    pub with_filters: Vec<std::any::TypeId>,
    pub without_filters: Vec<std::any::TypeId>,
    pub res_reads: Vec<std::any::TypeId>,
    pub res_writes: Vec<std::any::TypeId>,
    pub commands_accessed: Vec<std::any::TypeId>,
    pub tracked_components: Vec<std::any::TypeId>,
}

pub trait SystemParam {
    fn get_access() -> ParamAccess;
    fn extract(world: &mut World) -> Self;
}

impl<Q: WorldQuery + 'static, F: Filter + 'static> SystemParam for Query<Q, F> {
    fn get_access() -> ParamAccess {
        let mut access = ParamAccess {
            ..Default::default()
        };
        Q::collect_access(&mut access.reads, &mut access.writes);
        F::collect_filter(&mut access.with_filters, &mut access.without_filters);
        F::collect_tracking(&mut access.tracked_components);

        access
    }

    fn extract(world: &mut World) -> Self {
        Query::<Q, F>::new(world)
    }
}

pub trait System {
    fn run(&self, world: &mut World);
}

pub struct FunctionSystem<Marker, F> {
    pub func: F,
    pub _marker: std::marker::PhantomData<Marker>,
}

impl<Marker, F> FunctionSystem<Marker, F> {
    pub fn new(func: F) -> Self {
        Self {
            func,
            _marker: std::marker::PhantomData,
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

    fn extract(world: &mut World) -> Self {
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

    fn extract(world: &mut World) -> Self {
        unsafe { Self::new(world) }
    }
}
