use crate::{
    extensions::{FunctionData, ParamAccess, SystemParam},
    world::storage::World,
};
use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

pub struct Res<'w, T: 'static> {
    val_ptr: *const T,
    _marker: PhantomData<(&'w (), T)>,
}

impl<'w, T: 'static> Res<'w, T> {
    pub(crate) unsafe fn new(world: &World) -> Self {
        let res = world.get_resource::<T>();
        Self {
            val_ptr: res as *const T,
            _marker: PhantomData,
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.val_ptr }
    }
}

impl<'w, T: 'static> std::ops::Deref for Res<'w, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

unsafe impl<'w, T: Send> Send for Res<'w, T> {}
unsafe impl<'w, T: Sync> Sync for Res<'w, T> {}

impl<'w, T: Debug> Debug for Res<'w, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "{:?}", (*self.val_ptr)) }
    }
}

impl<'w, T: Display> Display for Res<'w, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "{}", (*self.val_ptr)) }
    }
}

impl<'w, T: 'static> SystemParam for Res<'w, T> {
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

pub struct ResMut<'w, T: 'static> {
    val_ptr: *mut T,
    _marker: PhantomData<(&'w (), T)>,
}

impl<'w, T: 'static> ResMut<'w, T> {
    pub(crate) unsafe fn new(world: &mut World) -> Self {
        let res = world.get_resource_mut::<T>();
        Self {
            val_ptr: res as *mut T,
            _marker: PhantomData,
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.val_ptr }
    }
}

impl<'w, T: 'static> std::ops::Deref for ResMut<'w, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.val_ptr }
    }
}

impl<'w, T: 'static> std::ops::DerefMut for ResMut<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

unsafe impl<'w, T: Send> Send for ResMut<'w, T> {}
unsafe impl<'w, T: Sync> Sync for ResMut<'w, T> {}

impl<'w, T: Debug> Debug for ResMut<'w, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "{:?}", (*self.val_ptr)) }
    }
}

impl<'w, T: Display> Display for ResMut<'w, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "{}", (*self.val_ptr)) }
    }
}

impl<'w, T: 'static> SystemParam for ResMut<'w, T> {
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

impl<'w, T: 'static> SystemParam for Option<Res<'w, T>> {
    fn get_access() -> ParamAccess {
        let mut access = ParamAccess {
            ..Default::default()
        };
        access.res_reads.push(std::any::TypeId::of::<T>());
        access
    }

    fn extract(world: &mut World, _system_data: &mut FunctionData) -> Self {
        let res_opt = world.get_resource_opt::<T>();
        if let Some(res) = res_opt {
            Some(Res {
                val_ptr: res as *const T,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }
}

impl<'w, T: 'static> SystemParam for Option<ResMut<'w, T>> {
    fn get_access() -> ParamAccess {
        let mut access = ParamAccess {
            ..Default::default()
        };
        access.res_writes.push(std::any::TypeId::of::<T>());
        access
    }

    fn extract(world: &mut World, _system_data: &mut FunctionData) -> Self {
        let res_opt = world.get_resource_mut_opt::<T>();
        if let Some(res) = res_opt {
            Some(ResMut {
                val_ptr: res as *mut T,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }
}
