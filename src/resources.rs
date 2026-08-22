use std::any::{TypeId, type_name};
use std::marker::PhantomData;

use crate::world::storage::World;

pub struct Res<T: 'static> {
    ptr: *const T,
    _marker: PhantomData<fn() -> &'static T>,
}

impl<T: 'static> Res<T> {
    pub unsafe fn new(world: &World) -> Self {
        let type_id = TypeId::of::<T>();
        let cell = world.resources.get(&type_id).unwrap_or_else(|| {
            panic!(
                "Requested resource: '{}' was never registered!",
                type_name::<T>()
            );
        });

        unsafe {
            let base_any = &mut *cell.get();
            let casted_ref = base_any
                .downcast_ref::<T>()
                .expect("Resource type mismatch!");
            Self {
                ptr: casted_ref as *const T,
                _marker: PhantomData,
            }
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T: 'static> std::ops::Deref for Res<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

pub struct ResMut<T: 'static> {
    ptr: *mut T,
    _marker: PhantomData<fn() -> &'static mut T>,
}

impl<T: 'static> ResMut<T> {
    pub unsafe fn new(world: &mut World) -> Self {
        let type_id = TypeId::of::<T>();
        let cell = world.resources.get_mut(&type_id).unwrap_or_else(|| {
            panic!(
                "Requested resource: '{}' was never registered!",
                type_name::<T>()
            );
        });
        let base_any = cell.get_mut();
        let casted_mut = base_any
            .downcast_mut::<T>()
            .expect("Resource type mismatch!");
        Self {
            ptr: casted_mut as *mut T,
            _marker: PhantomData,
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}

impl<T: 'static> std::ops::Deref for ResMut<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T: 'static> std::ops::DerefMut for ResMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
