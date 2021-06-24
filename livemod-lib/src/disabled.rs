use std::ops::{Deref, DerefMut};

use crate::LiveMod;

#[derive(Clone)]
pub struct LiveModHandle;

impl LiveModHandle {
    pub fn new_gui() -> LiveModHandle {
        LiveModHandle
    }

    pub fn new_with_ui(_: &str) -> LiveModHandle {
        LiveModHandle
    }

    pub fn create_variable<T: 'static + LiveMod>(&self, _: &str, var: T) -> ModVar<T> {
        ModVar {
            value: var,
        }
    }
}

#[repr(transparent)]
pub struct ModVar<T> {
    value: T
}

impl<T> ModVar<T> {
    pub fn lock(&self) -> ModVarGuard<T> {
        ModVarGuard(&self.value)
    }

    pub fn lock_mut(&mut self) -> ModVarMutGuard<T> {
        ModVarMutGuard(&mut self.value)
    }
}

pub struct ModVarGuard<'a, T>(&'a T);

impl<'a, T> Deref for ModVarGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

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