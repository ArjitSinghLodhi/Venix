use crate::world::storage::World;
use std::marker::PhantomData;

pub struct Res<T: 'static> {
    ptr: *const T,
    _marker: PhantomData<T>,
}

impl<T: 'static> Res<T> {
    pub(crate) unsafe fn new(world: &World) -> Self {
        let res = world.get_resource::<T>();
        Self {
            ptr: res as *const T,
            _marker: PhantomData,
        }
    }

    pub fn from_ref(reference: &T) -> Self {
        Res {
            ptr: reference as *const T,
            _marker: PhantomData,
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
    _marker: PhantomData<T>,
}

impl<T: 'static> ResMut<T> {
    pub(crate) unsafe fn new(world: &mut World) -> Self {
        let res = world.get_resource_mut::<T>();
        Self {
            ptr: res as *mut T,
            _marker: PhantomData,
        }
    }
    pub fn from_ref(reference: &mut T) -> Self {
        ResMut {
            ptr: reference as *mut T,
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
