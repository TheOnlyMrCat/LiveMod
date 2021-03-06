use std::ops::{Deref, DerefMut};

use crate::LiveMod;

pub struct LiveModHandle;

impl LiveModHandle {
    #[inline(always)]
    pub fn new_gui() -> LiveModHandle {
        LiveModHandle
    }

    #[inline(always)]
    pub fn new_with_ui(_: &str) -> LiveModHandle {
        LiveModHandle
    }

    #[inline(always)]
    pub fn track_variable<T: 'static + LiveMod>(&self, _: &str, _: &'static StaticModVar<T>) {}

    #[inline(always)]
    pub fn create_variable<T: 'static + LiveMod>(&self, _: &str, var: T) -> ModVar<T> {
        ModVar { value: var }
    }

    #[inline(always)]
    pub unsafe fn create_variable_unchecked<'a, T: 'a + LiveMod>(
        &self,
        _: &str,
        var: T,
    ) -> ModVar<T> {
        ModVar { value: var }
    }
}

#[repr(transparent)]
pub struct ModVar<T> {
    value: T,
}

impl<T> ModVar<T> {
    #[inline(always)]
    pub fn lock(&self) -> ModVarGuard<T> {
        ModVarGuard(&self.value)
    }

    #[inline(always)]
    pub fn lock_mut(&mut self) -> ModVarMutGuard<T> {
        ModVarMutGuard(&mut self.value)
    }
}

#[repr(transparent)]
pub struct StaticModVar<T> {
    value: T,
}

impl<T> StaticModVar<T> {
    pub const fn new(value: T) -> StaticModVar<T> {
        StaticModVar { value }
    }

    #[inline(always)]
    pub fn lock(&self) -> ModVarGuard<T> {
        ModVarGuard(&self.value)
    }
}

#[repr(transparent)]
pub struct ModVarGuard<'a, T>(&'a T);

impl<'a, T> Deref for ModVarGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[repr(transparent)]
pub struct ModVarMutGuard<'a, T>(&'a mut T);

impl<'a, T> Deref for ModVarMutGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T> DerefMut for ModVarMutGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}
