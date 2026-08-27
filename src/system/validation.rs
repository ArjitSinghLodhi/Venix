use std::{
    any::{Any, TypeId},
    hash::{BuildHasherDefault, Hash},
};

use fxhash::{FxHashMap, FxHashSet};

use crate::world::storage::World;

pub struct AccessHashSet<T: Eq + Hash> {
    pub(crate) set: FxHashSet<T>,
}

impl<T: Eq + Hash> AccessHashSet<T> {
    pub(crate) fn new() -> Self {
        Self {
            set: FxHashSet::default(),
        }
    }

    pub fn insert(&mut self, val: T) -> bool {
        self.set.insert(val)
    }

    pub fn contains(&self, val: &T) -> bool {
        self.set.contains(val)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.set.iter()
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
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.vec.iter()
    }
    pub fn as_slice(&self) -> &[T] {
        self.vec.as_slice()
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

pub trait System: SystemData {
    fn run(&mut self, world: &mut World);
}

pub trait SystemData {
    fn get_raw_data(&self, id: TypeId) -> Option<&Box<dyn Any>>;
    fn get_raw_data_mut(&mut self, id: TypeId) -> Option<&mut Box<dyn Any>>;
    fn insert_raw_data(&mut self, id: TypeId, value: Box<dyn Any>);
}

pub trait SystemExt {
    fn get_data<T: 'static>(&self) -> Option<&T>;
    fn get_data_mut<T: 'static>(&mut self) -> Option<&mut T>;
    fn insert<T: 'static>(&mut self, value: T);
    fn get_or_init<T: 'static>(&mut self, init: fn() -> T) -> &T;
    fn get_or_init_mut<T: 'static>(&mut self, init: fn() -> T) -> &mut T;
}

impl<S: SystemData + ?Sized> SystemExt for S {
    fn get_data<T: 'static>(&self) -> Option<&T> {
        self.get_raw_data(TypeId::of::<T>())
            .and_then(|any| any.downcast_ref::<T>())
    }

    fn get_data_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.get_raw_data_mut(TypeId::of::<T>())
            .and_then(|any| any.downcast_mut::<T>())
    }

    fn insert<T: 'static>(&mut self, value: T) {
        self.insert_raw_data(TypeId::of::<T>(), Box::new(value));
    }

    fn get_or_init<T: 'static>(&mut self, init: fn() -> T) -> &T {
        let id = TypeId::of::<T>();
        if self.get_raw_data(id).is_none() {
            self.insert_raw_data(id, Box::new(init()));
        }
        self.get_data::<T>().unwrap()
    }

    fn get_or_init_mut<T: 'static>(&mut self, init: fn() -> T) -> &mut T {
        let id = TypeId::of::<T>();
        if self.get_raw_data(id).is_none() {
            self.insert_raw_data(id, Box::new(init()));
        }
        self.get_data_mut::<T>().unwrap()
    }
}

impl SystemExt for Box<dyn System> {
    fn get_data<T: 'static>(&self) -> Option<&T> {
        (**self)
            .get_raw_data(TypeId::of::<T>())
            .and_then(|any| any.downcast_ref::<T>())
    }

    fn get_data_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (**self)
            .get_raw_data_mut(TypeId::of::<T>())
            .and_then(|any| any.downcast_mut::<T>())
    }

    fn insert<T: 'static>(&mut self, value: T) {
        (**self).insert_raw_data(TypeId::of::<T>(), Box::new(value));
    }

    fn get_or_init<T: 'static>(&mut self, init: fn() -> T) -> &T {
        let id = TypeId::of::<T>();
        if (**self).get_raw_data(id).is_none() {
            (**self).insert_raw_data(id, Box::new(init()));
        }
        self.get_data::<T>().unwrap()
    }

    fn get_or_init_mut<T: 'static>(&mut self, init: fn() -> T) -> &mut T {
        let id = TypeId::of::<T>();
        if (**self).get_raw_data(id).is_none() {
            (**self).insert_raw_data(id, Box::new(init()));
        }
        self.get_data_mut::<T>().unwrap()
    }
}

#[derive(Debug)]
pub struct FunctionData {
    data: FxHashMap<TypeId, Box<dyn Any>>,
}

impl FunctionData {
    pub(crate) fn new() -> Self {
        Self {
            data: FxHashMap::with_hasher(BuildHasherDefault::new()),
        }
    }
    pub fn get_data<T: 'static>(&self) -> Option<&T> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|any| any.downcast_ref::<T>())
    }

    pub fn get_data_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.data
            .get_mut(&TypeId::of::<T>())
            .and_then(|any| any.downcast_mut::<T>())
    }
    pub fn get_or_init<T: 'static, F: FnOnce() -> T>(&mut self, init: F) -> &T {
        let id = TypeId::of::<T>();
        let entry = self.data.entry(id).or_insert_with(|| Box::new(init()));
        entry.downcast_ref::<T>().unwrap()
    }

    pub fn get_or_init_mut<T: 'static, F: FnOnce() -> T>(&mut self, init: F) -> &mut T {
        let id = TypeId::of::<T>();
        let entry = self.data.entry(id).or_insert_with(|| Box::new(init()));
        entry.downcast_mut::<T>().unwrap()
    }
    pub fn insert<T: 'static>(&mut self, value: T) {
        let id = std::any::TypeId::of::<T>();
        self.data.insert(id, Box::new(value));
    }

    pub(crate) fn get_raw_data(&self, type_id: &TypeId) -> Option<&Box<dyn Any>> {
        self.data.get(type_id)
    }

    pub(crate) fn get_raw_data_mut(&mut self, type_id: &TypeId) -> Option<&mut Box<dyn Any>> {
        self.data.get_mut(type_id)
    }
    pub(crate) fn insert_raw_data(&mut self, type_id: TypeId, value: Box<dyn Any>) {
        self.data.insert(type_id, value);
    }
}

#[derive(Debug)]
pub struct FunctionSystem<Marker, F> {
    pub(crate) func: F,
    pub(crate) data: FunctionData,
    pub(crate) _marker: std::marker::PhantomData<Marker>,
}

impl<Marker, F> FunctionSystem<Marker, F> {
    pub(crate) fn new(func: F) -> Self {
        Self {
            func,
            data: FunctionData::new(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<Marker, F> SystemData for FunctionSystem<Marker, F> {
    fn get_raw_data(&self, id: TypeId) -> Option<&Box<dyn Any>> {
        self.data.get_raw_data(&id)
    }

    fn get_raw_data_mut(&mut self, id: TypeId) -> Option<&mut Box<dyn Any>> {
        self.data.get_raw_data_mut(&id)
    }

    fn insert_raw_data(&mut self, id: TypeId, value: Box<dyn Any>) {
        self.data.insert_raw_data(id, value);
    }
}

pub struct FunctionGenerationData {
    pub(crate) last_run_generation: u8,
    pub(crate) current_run_generation: u8,
    pub(crate) system_id: u32,
}

impl FunctionGenerationData {
    pub(crate) fn new() -> Self {
        Self {
            last_run_generation: 0,
            current_run_generation: 0,
            system_id: 0,
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

pub struct SystemId(u32);

impl SystemId {
    pub fn get_id(&self) -> u32 {
        self.0
    }
}

impl SystemParam for SystemId {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(_world: &mut World, system_data: &mut FunctionData) -> Self {
        let generation_data = system_data.get_data::<FunctionGenerationData>().unwrap();
        Self(generation_data.system_id)
    }
}
