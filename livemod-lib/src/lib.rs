//! # livemod

use std::ops::RangeInclusive;

#[cfg(feature = "livemod-derive")]
pub use livemod_derive::LiveMod;

#[cfg_attr(not(feature = "disabled"), allow(dead_code))]
mod disabled;
#[cfg_attr(feature = "disabled", allow(dead_code))]
mod enabled;

#[cfg(not(feature = "disabled"))]
pub use enabled::*;

#[cfg(feature = "disabled")]
pub use disabled::*;

use nanoserde::{DeBin, SerBin};

/// A named field in a tracked variable
#[derive(SerBin, DeBin)]
pub struct TrackedData {
    pub name: String,
    pub data_type: TrackedDataRepr,
}

/// The representation of a tracked field
#[derive(SerBin, DeBin)]
pub enum TrackedDataRepr {
    SignedSlider {
        storage_min: i64,
        storage_max: i64,
        suggested_min: i64,
        suggested_max: i64,
    },
    UnsignedSlider {
        storage_min: u64,
        storage_max: u64,
        suggested_min: u64,
        suggested_max: u64,
    },
    FloatSlider {
        storage_min: f64,
        storage_max: f64,
        suggested_min: f64,
        suggested_max: f64,
    },
    SignedInteger {
        min: i64,
        max: i64,
    },
    UnsignedInteger {
        min: u64,
        max: u64,
    },
    Float {
        min: f64,
        max: f64,
    },
    Bool,
    String {
        multiline: bool,
    },
    Struct {
        name: String,
        fields: Vec<TrackedData>,
    },
}

#[derive(SerBin, DeBin)]
pub enum TrackedDataValue {
    SignedInt(i64),
    UnsignedInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
}

impl TrackedDataValue {
    pub fn as_signed_int(&self) -> Option<&i64> {
        if let Self::SignedInt(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_unsigned_int(&self) -> Option<&u64> {
        if let Self::UnsignedInt(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<&f64> {
        if let Self::Float(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<&bool> {
        if let Self::Bool(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn into_string(self) -> Option<String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// Data which can be registered with a [`LiveModHandle`]
pub trait LiveMod {
    /// The default representation of the data
    fn data_type(&self) -> TrackedDataRepr;
    /// If this is a struct, get the field by this name
    fn get_named_value(&mut self, name: &str) -> &mut dyn LiveMod;
    /// If this is a value, set self to the given value
    fn set_self(&mut self, value: TrackedDataValue);
}

pub trait Slider {
    fn repr_slider(&self, range: RangeInclusive<Self>) -> TrackedDataRepr
    where
        Self: Sized;
}

pub trait Multiline {
    fn repr_multiline(&self) -> TrackedDataRepr;
}

#[macro_export]
macro_rules! livemod_static {
    ($($vis:vis static $name:ident : $ty:ty = $val:expr;)*) => {
        $(
        $vis static $name: $crate::StaticModVar<$ty> = $crate::StaticModVar::new($val);
        )*
    }
}

macro_rules! unsigned_primitive_impl {
    ($($ty:ident),*) => {
        $(
        impl LiveMod for $ty {
            fn data_type(&self) -> TrackedDataRepr {
                TrackedDataRepr::UnsignedInteger {
                    min: $ty::MIN as u64,
                    max: $ty::MAX as u64,
                }
            }

            fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
                unimplemented!()
            }

            fn set_self(&mut self, value: TrackedDataValue) {
                *self = *value.as_unsigned_int().unwrap() as $ty
            }
        }

        impl Slider for $ty {
            fn repr_slider(&self, range: RangeInclusive<Self>) -> TrackedDataRepr {
                TrackedDataRepr::UnsignedSlider {
                    storage_min: $ty::MIN as u64,
                    storage_max: $ty::MAX as u64,
                    suggested_min: *range.start() as u64,
                    suggested_max: *range.end() as u64,
                }
            }
        }
        )*
    }
}

macro_rules! signed_primitive_impl {
    ($($ty:ident),*) => {
        $(
        impl LiveMod for $ty {
            fn data_type(&self) -> TrackedDataRepr {
                TrackedDataRepr::SignedInteger {
                    min: $ty::MIN as i64,
                    max: $ty::MAX as i64,
                }
            }

            fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
                unimplemented!()
            }

            fn set_self(&mut self, value: TrackedDataValue) {
                *self = *value.as_signed_int().unwrap() as $ty
            }
        }

        impl Slider for $ty {
            fn repr_slider(&self, range: RangeInclusive<Self>) -> TrackedDataRepr {
                TrackedDataRepr::SignedSlider {
                    storage_min: $ty::MIN as i64,
                    storage_max: $ty::MAX as i64,
                    suggested_min: *range.start() as i64,
                    suggested_max: *range.end() as i64,
                }
            }
        }
        )*
    }
}

macro_rules! float_primitive_impl {
    ($($ty:ident),*) => {
        $(
        impl LiveMod for $ty {
            fn data_type(&self) -> TrackedDataRepr {
                TrackedDataRepr::Float {
                    min: $ty::MIN as f64,
                    max: $ty::MAX as f64,
                }
            }

            fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
                unimplemented!()
            }

            fn set_self(&mut self, value: TrackedDataValue) {
                *self = *value.as_float().unwrap() as $ty
            }
        }

        impl Slider for $ty {
            fn repr_slider(&self, range: RangeInclusive<Self>) -> TrackedDataRepr {
                TrackedDataRepr::FloatSlider {
                    storage_min: $ty::MIN as f64,
                    storage_max: $ty::MAX as f64,
                    suggested_min: *range.start() as f64,
                    suggested_max: *range.end() as f64,
                }
            }
        }
        )*
    }
}

unsigned_primitive_impl!(u8, u16, u32, u64, usize);
signed_primitive_impl!(i8, i16, i32, i64, isize);
float_primitive_impl!(f32, f64);

impl LiveMod for bool {
    fn data_type(&self) -> TrackedDataRepr {
        TrackedDataRepr::Bool
    }

    fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
        unimplemented!()
    }

    fn set_self(&mut self, value: TrackedDataValue) {
        *self = *value.as_bool().unwrap()
    }
}

impl LiveMod for String {
    fn data_type(&self) -> TrackedDataRepr {
        TrackedDataRepr::String {
            multiline: false,
        }
    }

    fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
        unimplemented!()
    }

    fn set_self(&mut self, value: TrackedDataValue) {
        *self = value.into_string().unwrap()
    }
}

impl Multiline for String {
    fn repr_multiline(&self) -> TrackedDataRepr {
        TrackedDataRepr::String {
            multiline: true
        }
    }
}