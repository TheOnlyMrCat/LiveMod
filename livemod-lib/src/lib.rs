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
#[derive(Clone, Debug, SerBin, DeBin)]
pub struct TrackedData {
    pub name: String,
    pub data_type: TrackedDataRepr,
    pub triggers: Vec<String>,
}

/// The representation of a tracked field
#[derive(Clone, Debug, SerBin, DeBin)]
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
    Trigger {
        name: String,
    },
    String {
        multiline: bool,
    },
    Enum {
        name: String,
        variants: Vec<String>,
        fields: Vec<TrackedData>,
        triggers: Vec<String>,
    },
    Struct {
        name: String,
        fields: Vec<TrackedData>,
        triggers: Vec<String>,
    },
}

#[derive(Debug, Clone, SerBin, DeBin)]
pub enum TrackedDataValue {
    SignedInt(i64),
    UnsignedInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Struct(Vec<(String, TrackedDataValue)>),
    EnumVariant(String),
    Enum {
        variant: String,
        fields: Vec<(String, TrackedDataValue)>,
    },
    Trigger,
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

    pub fn try_into_enum_variant(self) -> Result<String, Self> {
        if let Self::EnumVariant(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }
}

#[derive(Clone, Debug)]
pub enum Trigger {
    Set(TrackedDataValue),
    Trigger(String),
}

impl Trigger {
    pub fn try_into_set(self) -> Result<TrackedDataValue, Self> {
        if let Self::Set(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_trigger(self) -> Result<String, Self> {
        if let Self::Trigger(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }
}

/// Data which can be registered with a [`LiveModHandle`]
pub trait LiveMod {
    /// The default representation of the data
    fn repr_default(&self) -> TrackedDataRepr;
    /// If this is a struct, get the field by this name
    fn get_named_value(&mut self, name: &str) -> &mut dyn LiveMod;
    /// Trigger an action on this data. Returns whether the representation has changed
    fn trigger(&mut self, trigger: Trigger) -> bool;
    /// Return the current value of this data, whether it is a struct or a value
    fn get_self(&self) -> TrackedDataValue;
}

/// Slider representation for numeric values
pub trait Slider {
    fn repr_slider(&self, range: RangeInclusive<Self>) -> TrackedDataRepr
    where
        Self: Sized;
}

/// Multiline string input
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
            fn repr_default(&self) -> TrackedDataRepr {
                TrackedDataRepr::UnsignedInteger {
                    min: $ty::MIN as u64,
                    max: $ty::MAX as u64,
                }
            }

            fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
                unimplemented!()
            }

            fn trigger(&mut self, trigger: Trigger) -> bool {
                *self = *trigger.try_into_set().unwrap().as_unsigned_int().unwrap() as $ty;
                false
            }

            fn get_self(&self) -> TrackedDataValue {
                TrackedDataValue::UnsignedInt(*self as u64)
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
            fn repr_default(&self) -> TrackedDataRepr {
                TrackedDataRepr::SignedInteger {
                    min: $ty::MIN as i64,
                    max: $ty::MAX as i64,
                }
            }

            fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
                unimplemented!()
            }

            fn trigger(&mut self, trigger: Trigger) -> bool {
                *self = *trigger.try_into_set().unwrap().as_signed_int().unwrap() as $ty;
                false
            }

            fn get_self(&self) -> TrackedDataValue {
                TrackedDataValue::SignedInt(*self as i64)
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
            fn repr_default(&self) -> TrackedDataRepr {
                TrackedDataRepr::Float {
                    min: $ty::MIN as f64,
                    max: $ty::MAX as f64,
                }
            }

            fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
                unimplemented!()
            }

            fn trigger(&mut self, trigger: Trigger) -> bool {
                *self = *trigger.try_into_set().unwrap().as_float().unwrap() as $ty;
                false
            }

            fn get_self(&self) -> TrackedDataValue {
                TrackedDataValue::Float(*self as f64)
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
    fn repr_default(&self) -> TrackedDataRepr {
        TrackedDataRepr::Bool
    }

    fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
        unimplemented!()
    }

    fn trigger(&mut self, trigger: Trigger) -> bool {
        *self = *trigger.try_into_set().unwrap().as_bool().unwrap();
        false
    }

    fn get_self(&self) -> TrackedDataValue {
        TrackedDataValue::Bool(*self)
    }
}

impl LiveMod for String {
    fn repr_default(&self) -> TrackedDataRepr {
        TrackedDataRepr::String { multiline: false }
    }

    fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
        unimplemented!()
    }

    fn trigger(&mut self, trigger: Trigger) -> bool {
        *self = trigger.try_into_set().unwrap().into_string().unwrap();
        false
    }

    fn get_self(&self) -> TrackedDataValue {
        TrackedDataValue::String(self.clone())
    }
}

impl Multiline for String {
    fn repr_multiline(&self) -> TrackedDataRepr {
        TrackedDataRepr::String { multiline: true }
    }
}

impl<T> LiveMod for Vec<T>
where
    T: LiveMod + Default,
{
    fn repr_default(&self) -> TrackedDataRepr {
        TrackedDataRepr::Struct {
            name: "Vec".to_owned(),
            fields: self
                .iter()
                .enumerate()
                .map(|(i, v)| TrackedData {
                    name: format!("{}", i),
                    data_type: v.repr_default(),
                    triggers: vec!["Remove".to_owned()],
                })
                .collect(),
            triggers: vec!["Add element".to_owned()],
        }
    }

    fn get_named_value(&mut self, name: &str) -> &mut dyn LiveMod {
        let idx = name.parse::<usize>().unwrap();
        &mut self[idx]
    }

    fn trigger(&mut self, trigger: Trigger) -> bool {
        let trigger = trigger.try_into_trigger().unwrap();
        if trigger == "Add element" {
            self.push(Default::default());
        } else {
            let parts = trigger.split('.').collect::<Vec<_>>();
            if parts[1] == "Remove" {
                self.remove(parts[0].parse().unwrap());
            }
        }
        true
    }

    fn get_self(&self) -> TrackedDataValue {
        TrackedDataValue::Struct(
            self.iter()
                .enumerate()
                .map(|(i, v)| (format!("{}", i), v.get_self()))
                .collect(),
        )
    }
}

pub struct TriggerFn<A, F: FnMut(&mut A)> {
    arg: A,
    func: F,
}

impl<A, F: FnMut(&mut A)> TriggerFn<A, F> {
    pub fn new(arg: A, func: F) -> TriggerFn<A, F> {
        TriggerFn { arg, func }
    }
}

impl<A, F: FnMut(&mut A)> LiveMod for TriggerFn<A, F> {
    fn repr_default(&self) -> TrackedDataRepr {
        TrackedDataRepr::Trigger {
            name: "Call".to_owned(),
        }
    }

    fn get_named_value(&mut self, _name: &str) -> &mut dyn LiveMod {
        unimplemented!()
    }

    fn trigger(&mut self, trigger: Trigger) -> bool {
        trigger.try_into_trigger().unwrap();
        (self.func)(&mut self.arg);
        false
    }

    fn get_self(&self) -> TrackedDataValue {
        TrackedDataValue::Trigger
    }
}
