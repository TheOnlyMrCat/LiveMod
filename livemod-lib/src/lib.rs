//! # livemod - Runtime modification of program parameters

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

/// A named field in a tracked structure
#[derive(Clone, Debug, SerBin, DeBin)]
pub struct TrackedData {
    /// The name of the field, to be displayed in the UI
    pub name: String,
    /// The representation of the field in the UI
    pub data_type: TrackedDataRepr,
    /// Triggers from the surrounding structure on this field. For example, remove an element from an array.
    pub triggers: Vec<String>,
}

/// The representation of a tracked field. Corresponds with one or more [`TrackedDataValue`]s
#[derive(Clone, Debug, SerBin, DeBin)]
pub enum TrackedDataRepr {
    /// A signed integer with suggested bounds, corresponds with [`TrackedDataValue::SignedInt`]
    SignedSlider {
        storage_min: i64,
        storage_max: i64,
        suggested_min: i64,
        suggested_max: i64,
    },
    /// An unsigned integer with suggested bounds, corresponds with [`TrackedDataValue::UnsignedInt`]
    UnsignedSlider {
        storage_min: u64,
        storage_max: u64,
        suggested_min: u64,
        suggested_max: u64,
    },
    /// A floating-point number with suggested bounds, corresponds with [`TrackedDataValue::Float`]
    FloatSlider {
        storage_min: f64,
        storage_max: f64,
        suggested_min: f64,
        suggested_max: f64,
    },
    /// A signed integer without suggested bounds, corresponds with [`TrackedDataValue::SignedInt`]
    SignedInteger { min: i64, max: i64 },
    /// An unsigned integer without suggested bounds, corresponds with [`TrackedDataValue::UnsignedInt`]
    UnsignedInteger { min: u64, max: u64 },
    /// A floating-point number without suggested bounds, corresponds with [`TrackedDataValue::Float`]
    Float { min: f64, max: f64 },
    /// A boolean, corresponds with [`TrackedDataValue::Bool`]
    Bool,
    /// A triggerable action, corresponds with [`TrackedDataValue::Trigger`]
    Trigger { name: String },
    /// A string, corresponds with [`TrackedDataValue::String`]
    String { multiline: bool },
    /// A sum type with named fields, corresponds with [`TrackedDataValue::Enum`] and [`TrackedDataValue::EnumVariant`]
    Enum {
        name: String,
        variants: Vec<String>,
        fields: Vec<TrackedData>,
        triggers: Vec<String>,
    },
    /// A heterogeneous structure with named fields, corresponds with [`TrackedDataValue::Struct`]
    Struct {
        name: String,
        fields: Vec<TrackedData>,
        triggers: Vec<String>,
    },
}

/// The data contained within a tracked field. Corresponds with one or more [`TrackedDataRepr`]s
#[derive(Debug, Clone, SerBin, DeBin)]
pub enum TrackedDataValue {
    /// A signed integer, corresponds with [`TrackedDataRepr::SignedSlider`] and [`TrackedDataRepr::SignedInteger`]
    SignedInt(i64),
    /// An unsigned integer, corresponds with [`TrackedDataRepr::UnsignedSlider`] and [`TrackedDataRepr::UnsignedInteger`]
    UnsignedInt(u64),
    /// A floating-point value, corresponds with [`TrackedDataRepr::FloatSlider`] and [`TrackedDataRepr::Float`]
    Float(f64),
    /// A boolean, corresponds with [`TrackedDataRepr::Bool`]
    Bool(bool),
    /// A string, corresponds with [`TrackedDataRepr::String`]
    String(String),
    /// A heterogeneous structure of labelled fields, corresponds with [`TrackedDataRepr::Struct`]
    Struct(Vec<(String, TrackedDataValue)>),
    /// The name of a variant in a sum type, corresponds with [`TrackedDataRepr::Enum`]
    EnumVariant(String),
    /// The variant and fields in a sum type, corresponds with [`TrackedDataRepr::Enum`]
    Enum {
        variant: String,
        fields: Vec<(String, TrackedDataValue)>,
    },
    /// No value, but does something when activated, corresponds with [`TrackedDataRepr::Trigger`]
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

/// An action that can be taken on a [`LiveMod`] variable.
#[derive(Clone, Debug)]
pub enum Trigger {
    /// Set its own value to this value.
    Set(TrackedDataValue),
    /// Trigger an action by the given name.
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
        if let Self::Trigger(s) = self {
            Ok(s)
        } else {
            Err(self)
        }
    }
}

/// The target of a method call on a [`LiveMod`] variable.
pub enum ActionTarget<'a, 'b> {
    /// The variable itself.
    This,
    /// A field of the variable.
    Field(&'a [&'b str]),
}

impl ActionTarget<'_, '_> {
    /// Returns `true` if the action_target is [`This`].
    pub fn is_this(&self) -> bool {
        matches!(self, Self::This)
    }

    /// Create an `ActionTarget` by stripping the first element off the given slice.
    pub fn from_name_and_fields<'a, 'b>(slice: &'a [&'b str]) -> ActionTarget<'a, 'b> {
        if slice.len() < 2 {
            ActionTarget::This
        } else {
            ActionTarget::Field(&slice[1..])
        }
    }

    /// Returns `true` if the action_target is [`Field`].
    pub fn strip_one_field(&self) -> Option<(&str, ActionTarget)> {
        match self {
            Self::This => None,
            Self::Field(fields) => {
                Some((fields[0], if fields.len() > 1 { ActionTarget::Field(&fields[1..]) } else { ActionTarget::This }))
            }
        }
    }
}

/// Data which can be registered with a [`LiveModHandle`]
pub trait LiveMod {
    /// The default representation of the data
    fn repr_default(&self, target: ActionTarget) -> TrackedDataRepr;
    /// Return the current value of this data, whether it is a struct or a value
    fn get_self(&self, target: ActionTarget) -> TrackedDataValue;
    /// Trigger an action on this data. Returns whether the representation has changed and needs to be updated.
    fn trigger(&mut self, target: ActionTarget, trigger: Trigger) -> bool;
}

pub trait LiveModRepr<T> {
    fn repr(&self, cur: &T) -> TrackedDataRepr;
}

pub struct DefaultRepr;

impl<T> LiveModRepr<T> for DefaultRepr
where
    T: LiveMod,
{
    fn repr(&self, cur: &T) -> TrackedDataRepr {
        cur.repr_default(ActionTarget::This)
    }
}

/// Slider representation for numeric values
pub struct Slider<T>(pub RangeInclusive<T>);

macro_rules! impl_slider {
    ($(($t:ty, $var:ident)),*) => {
        $(
            impl LiveModRepr<$t> for Slider<$t> {
                fn repr(&self, _cur: &$t) -> TrackedDataRepr {
                    TrackedDataRepr::$var {
                        suggested_min: (*self.0.start()).into(),
                        suggested_max: (*self.0.end()).into(),
                        storage_min: <$t>::MIN.into(),
                        storage_max: <$t>::MAX.into(),
                    }
                }
            }
        )*
    };
}

impl_slider!(
    (i8, SignedSlider),
    (i16, SignedSlider),
    (i32, SignedSlider),
    (i64, SignedSlider),
    (u8, UnsignedSlider),
    (u16, UnsignedSlider),
    (u32, UnsignedSlider),
    (u64, UnsignedSlider),
    (f32, FloatSlider),
    (f64, FloatSlider)
);

/// Multiline string input
pub struct Multiline;

impl LiveModRepr<String> for Multiline {
    fn repr(&self, _cur: &String) -> TrackedDataRepr {
        TrackedDataRepr::String {
            multiline: true,
        }
    }
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
            fn repr_default(&self, target: ActionTarget) -> TrackedDataRepr {
                debug_assert!(target.is_this());
                TrackedDataRepr::UnsignedInteger {
                    min: $ty::MIN as u64,
                    max: $ty::MAX as u64,
                }
            }

            fn trigger(&mut self, target: ActionTarget, trigger: Trigger) -> bool {
                debug_assert!(target.is_this());
                *self = *trigger.try_into_set().unwrap().as_unsigned_int().unwrap() as $ty;
                false
            }

            fn get_self(&self, target: ActionTarget) -> TrackedDataValue {
                debug_assert!(target.is_this());
                TrackedDataValue::UnsignedInt(*self as u64)
            }
        }
        )*
    }
}

macro_rules! signed_primitive_impl {
    ($($ty:ident),*) => {
        $(
        impl LiveMod for $ty {
            fn repr_default(&self, target: ActionTarget) -> TrackedDataRepr {
                debug_assert!(target.is_this());
                TrackedDataRepr::SignedInteger {
                    min: $ty::MIN as i64,
                    max: $ty::MAX as i64,
                }
            }

            fn trigger(&mut self, target: ActionTarget, trigger: Trigger) -> bool {
                debug_assert!(target.is_this());
                *self = *trigger.try_into_set().unwrap().as_signed_int().unwrap() as $ty;
                false
            }

            fn get_self(&self, target: ActionTarget) -> TrackedDataValue {
                debug_assert!(target.is_this());
                TrackedDataValue::SignedInt(*self as i64)
            }
        }
        )*
    }
}

macro_rules! float_primitive_impl {
    ($($ty:ident),*) => {
        $(
        impl LiveMod for $ty {
            fn repr_default(&self, target: ActionTarget) -> TrackedDataRepr {
                debug_assert!(target.is_this());
                TrackedDataRepr::Float {
                    min: $ty::MIN as f64,
                    max: $ty::MAX as f64,
                }
            }

            fn trigger(&mut self, target: ActionTarget, trigger: Trigger) -> bool {
                debug_assert!(target.is_this());
                *self = *trigger.try_into_set().unwrap().as_float().unwrap() as $ty;
                false
            }

            fn get_self(&self, target: ActionTarget) -> TrackedDataValue {
                debug_assert!(target.is_this());
                TrackedDataValue::Float(*self as f64)
            }
        }
        )*
    }
}

unsigned_primitive_impl!(u8, u16, u32, u64, usize);
signed_primitive_impl!(i8, i16, i32, i64, isize);
float_primitive_impl!(f32, f64);

impl LiveMod for bool {
    fn repr_default(&self, target: ActionTarget) -> TrackedDataRepr {
        TrackedDataRepr::Bool
    }

    fn trigger(&mut self, target: ActionTarget, trigger: Trigger) -> bool {
        *self = *trigger.try_into_set().unwrap().as_bool().unwrap();
        false
    }

    fn get_self(&self, target: ActionTarget) -> TrackedDataValue {
        TrackedDataValue::Bool(*self)
    }
}

impl LiveMod for String {
    fn repr_default(&self, target: ActionTarget) -> TrackedDataRepr {
        TrackedDataRepr::String { multiline: false }
    }

    fn trigger(&mut self, target: ActionTarget, trigger: Trigger) -> bool {
        *self = trigger.try_into_set().unwrap().into_string().unwrap();
        false
    }

    fn get_self(&self, target: ActionTarget) -> TrackedDataValue {
        TrackedDataValue::String(self.clone())
    }
}

impl<T> LiveMod for Vec<T>
where
    T: LiveMod + Default,
{
    fn repr_default(&self, target: ActionTarget) -> TrackedDataRepr {
        if let Some((field, field_target)) = target.strip_one_field() {
            self[field.parse::<usize>().unwrap()].repr_default(field_target)
        } else {
            TrackedDataRepr::Struct {
                name: "Vec".to_owned(),
                fields: self
                    .iter()
                    .enumerate()
                    .map(|(i, v)| TrackedData {
                        name: format!("{}", i),
                        data_type: v.repr_default(ActionTarget::This),
                        triggers: vec!["Remove".to_owned()],
                    })
                    .collect(),
                triggers: vec!["Add element".to_owned()],
            }
        }
    }

    fn trigger(&mut self, target: ActionTarget, trigger: Trigger) -> bool {
        if let Some((field, field_target)) = target.strip_one_field() {
            self[field.parse::<usize>().unwrap()].trigger(field_target, trigger)
        } else {
            match trigger {
                Trigger::Trigger(trigger) => {
                    if trigger == "Add element" {
                        self.push(Default::default());
                    } else {
                        let parts = trigger.split('.').collect::<Vec<_>>();
                        if parts[1] == "Remove" {
                            self.remove(parts[0].parse().unwrap());
                        }
                    }
                },
                _ => panic!(),
            }
            true
        }
    }

    fn get_self(&self, target: ActionTarget) -> TrackedDataValue {
        if let Some((field, field_target)) = target.strip_one_field() {
            self[field.parse::<usize>().unwrap()].get_self(field_target)
        } else {
            TrackedDataValue::Struct(
                self.iter()
                    .enumerate()
                    .map(|(i, v)| (format!("{}", i), v.get_self(ActionTarget::This)))
                    .collect(),
            )
        }
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
    fn repr_default(&self, target: ActionTarget) -> TrackedDataRepr {
        TrackedDataRepr::Trigger {
            name: "Call".to_owned(),
        }
    }

    fn trigger(&mut self, target: ActionTarget, trigger: Trigger) -> bool {
        trigger.try_into_trigger().unwrap();
        (self.func)(&mut self.arg);
        false
    }

    fn get_self(&self, target: ActionTarget) -> TrackedDataValue {
        TrackedDataValue::Trigger
    }
}
