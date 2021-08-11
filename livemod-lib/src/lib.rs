//! # livemod - Runtime modification of program parameters

use std::array::IntoIter;
use std::collections::HashMap;
use std::iter::FromIterator;
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

/// Convenience type to create builtin livemod reprs.
#[derive(Clone, Debug)]
pub enum BuiltinRepr {
    /// A signed integer with suggested bounds.
    ///
    /// Maps to `livemod:sint`
    SignedSlider {
        storage_min: i64,
        storage_max: i64,
        suggested_min: i64,
        suggested_max: i64,
    },
    /// An unsigned integer with suggested bounds.
    ///
    /// Maps to `livemod:uint`
    UnsignedSlider {
        storage_min: u64,
        storage_max: u64,
        suggested_min: u64,
        suggested_max: u64,
    },
    /// A floating-point number with suggested bounds
    ///
    /// Maps to `livemod:float`
    FloatSlider {
        storage_min: f64,
        storage_max: f64,
        suggested_min: f64,
        suggested_max: f64,
    },
    /// A signed integer without suggested bounds.
    ///
    /// Maps to `livemod:sint`
    SignedInteger { min: i64, max: i64 },
    /// An unsigned integer without suggested bounds
    ///
    /// Maps to `livemod:uint`
    UnsignedInteger { min: u64, max: u64 },
    /// A floating-point number without suggested bounds
    ///
    /// Maps to `livemod:float`
    Float { min: f64, max: f64 },
    /// A boolean
    ///
    /// Maps to `livemod:bool`
    Bool,
    /// A string
    ///
    /// Maps to `livemod:string`
    String { multiline: bool },
}

#[derive(Clone, Copy, Debug)]
pub struct Repr;
#[derive(Clone, Copy, Debug)]
pub struct Value;

#[derive(Clone, Debug)]
pub enum Parameter<T> {
    SignedInt(i64),
    UnsignedInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Namespaced(Namespaced<T>),
}

impl<T> Parameter<T> {
    pub fn serialize(&self) -> String {
        match self {
            Parameter::SignedInt(i) => format!("{:+}", i),
            Parameter::UnsignedInt(i) => format!("{}", i),
            Parameter::Float(f) => format!("d{}", f),
            Parameter::Bool(true) => "t".to_owned(),
            Parameter::Bool(false) => "f".to_owned(),
            Parameter::String(s) => format!("\"{}\"", s),
            Parameter::Namespaced(n) => format!("{}", n.serialize()),
        }
    }

    pub fn deserialize(s: &str) -> Result<Parameter<T>, ()> {
        //TODO: Should it be a result?
        match s.bytes().next().unwrap() {
            b'-' | b'+' => Ok(Parameter::SignedInt(s.parse().unwrap())),
            b'd' => Ok(Parameter::Float(s[1..].parse().unwrap())),
            c if c.is_ascii_digit() => Ok(Parameter::UnsignedInt(s.parse().unwrap())),
            b't' => Ok(Parameter::Bool(true)),
            b'f' => Ok(Parameter::Bool(false)),
            b'\"' => Ok(Parameter::String(s[1..s.len() - 2].to_owned())),
            _ => Ok(Parameter::Namespaced(Namespaced::deserialize(&s)?))
        }
    }

    pub fn try_into_signed_int(self) -> Result<i64, Self> {
        if let Self::SignedInt(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_unsigned_int(self) -> Result<u64, Self> {
        if let Self::UnsignedInt(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_float(self) -> Result<f64, Self> {
        if let Self::Float(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_bool(self) -> Result<bool, Self> {
        if let Self::Bool(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_string(self) -> Result<String, Self> {
        if let Self::String(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_namespaced(self) -> Result<Namespaced<T>, Self> {
        if let Self::Namespaced(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Namespaced<T> {
    pub name: Vec<String>,
    pub parameters: HashMap<String, Parameter<T>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Namespaced<T> {
    pub fn new(name: Vec<String>, parameters: HashMap<String, Parameter<T>>) -> Self {
        Namespaced {
            name,
            parameters,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn serialize(&self) -> String {
        let mut s = self.name[..].join(":");
        s.push('{');
        for (k, v) in self.parameters.iter() {
            s.push_str(k);
            s.push('=');
            s.push_str(&v.serialize());
            s.push(',');
        };
        s.push_str("}");
        s
    }

    pub fn deserialize(s: &str) -> Result<Namespaced<T>, ()> {
        let (name, params) = s.split_once('{').unwrap(); // Rust 1.52... Should I support earlier?
        let name = name.split(':').map(|s| s.trim().to_owned()).collect();

        let mut parameters = HashMap::new();
        for s in params.split(',') {
            if s.is_empty() {
                continue;
            }
            let (k, v) = s.split_once('=').unwrap();
            let parameter = Parameter::deserialize(&v)?;
            parameters.insert(k.to_owned(), parameter);
        }

        Ok(Namespaced {
            name,
            parameters,
            _marker: std::marker::PhantomData,
        })
    }
}
impl Namespaced<Repr> {
    pub fn new_basic_structure(
        name: &str,
        fields: &[(String, Namespaced<Repr>)],
    ) -> Namespaced<Repr> {
        Namespaced {
            name: vec!["livemod".to_owned(), "struct".to_owned()],
            parameters: HashMap::from_iter(IntoIter::new([
                ("name".to_owned(), Parameter::String(name.to_owned())),
                (
                    "fields".to_owned(),
                    Parameter::Namespaced(Namespaced {
                        name: vec!["livemod".to_owned(), "fields".to_owned()],
                        parameters: HashMap::from_iter(
                            fields.iter().map(|(name, field)| {
                                (name.to_owned(), Parameter::Namespaced(field.clone()))
                            }),
                        ),
                        _marker: std::marker::PhantomData,
                    }),
                ),
            ])),
            _marker: std::marker::PhantomData,
        }
    }
}

impl Namespaced<Value> {
    pub fn new_basic_structure(
        fields: &[(String, Namespaced<Value>)],
    ) -> Namespaced<Value> {
        Namespaced {
            name: vec!["livemod".to_owned(), "struct".to_owned()],
            parameters: HashMap::from_iter(
                fields
                    .iter()
                    .map(|(name, field)| (name.to_owned(), Parameter::Namespaced(field.clone()))),
            ),
            _marker: std::marker::PhantomData,
        }
    }
}

impl From<BuiltinRepr> for Namespaced<Repr> {
    fn from(b: BuiltinRepr) -> Namespaced<Repr> {
        match b {
            BuiltinRepr::Bool => Namespaced {
                name: vec!["livemod".to_owned(), "bool".to_owned()],
                parameters: HashMap::new(),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::String { multiline } => Namespaced {
                name: vec!["livemod".to_owned(), "string".to_owned()],
                parameters: HashMap::from_iter(IntoIter::new([(
                    "multiline".to_owned(),
                    Parameter::Bool(multiline),
                )])),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::SignedInteger { min, max } => Namespaced {
                name: vec!["livemod".to_owned(), "sint".to_owned()],
                parameters: HashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::SignedInt(min)),
                    ("max".to_owned(), Parameter::SignedInt(max)),
                ])),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::UnsignedInteger { min, max } => Namespaced {
                name: vec!["livemod".to_owned(), "uint".to_owned()],
                parameters: HashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::UnsignedInt(min)),
                    ("max".to_owned(), Parameter::UnsignedInt(max)),
                ])),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::Float { min, max } => Namespaced {
                name: vec!["livemod".to_owned(), "float".to_owned()],
                parameters: HashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::Float(min)),
                    ("max".to_owned(), Parameter::Float(max)),
                ])),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::SignedSlider {
                storage_min,
                storage_max,
                suggested_min,
                suggested_max,
            } => Namespaced {
                name: vec!["livemod".to_owned(), "sint".to_owned()],
                parameters: HashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::SignedInt(storage_min)),
                    ("max".to_owned(), Parameter::SignedInt(storage_max)),
                    ("suggested_min".to_owned(), Parameter::SignedInt(suggested_min)),
                    ("suggested_max".to_owned(), Parameter::SignedInt(suggested_max)),
                ])),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::UnsignedSlider {
                storage_min,
                storage_max,
                suggested_min,
                suggested_max,
            } => Namespaced {
                name: vec!["livemod".to_owned(), "uint".to_owned()],
                parameters: HashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::UnsignedInt(storage_min)),
                    ("max".to_owned(), Parameter::UnsignedInt(storage_max)),
                    ("suggested_min".to_owned(), Parameter::UnsignedInt(suggested_min)),
                    ("suggested_max".to_owned(), Parameter::UnsignedInt(suggested_max)),
                ])),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::FloatSlider {
                storage_min,
                storage_max,
                suggested_min,
                suggested_max,
            } => Namespaced {
                name: vec!["livemod".to_owned(), "float".to_owned()],
                parameters: HashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::Float(storage_min)),
                    ("max".to_owned(), Parameter::Float(storage_max)),
                    ("suggested_min".to_owned(), Parameter::Float(suggested_min)),
                    ("suggested_max".to_owned(), Parameter::Float(suggested_max)),
                ])),
                _marker: std::marker::PhantomData,
            },
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
    ///
    /// ```
    /// assert_eq!(ActionTarget::from_name_and_fields(["foo"]), ActionTarget::This);
    /// assert_eq!(ActionTarget::from_name_and_fields(["foo", "bar"]), ActionTarget::Field(&["bar"]));
    /// ```
    pub fn from_name_and_fields<'a, 'b>(slice: &'a [&'b str]) -> ActionTarget<'a, 'b> {
        if slice.len() < 2 {
            ActionTarget::This
        } else {
            ActionTarget::Field(&slice[1..])
        }
    }

    /// If this is a `ActionTarget::Field`, return the topmost field name and the `ActionTarget` to use when calling the field.
    /// Otherwise, return `None`.
    ///
    /// ```
    /// assert_eq!(ActionTarget::This.strip_one_field(), None);
    /// assert_eq!(ActionTarget::Field(&["foo"]).strip_one_field(), Some(("foo", ActionTarget::This)));
    /// assert_eq!(ActionTarget::Field(&["foo", "bar"]).strip_one_field(), Some(("foo", ActionTarget::Field(&["bar"]))));
    /// ```
    pub fn strip_one_field(&self) -> Option<(&str, ActionTarget<'_, '_>)> {
        match self {
            Self::This => None,
            Self::Field(fields) => Some((fields[0], ActionTarget::from_name_and_fields(fields))),
        }
    }
}

/// Data which can be registered with a [`LiveModHandle`]
pub trait LiveMod {
    /// The default representation of the data
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr>;
    /// Return the current value of this data, whether it is a struct or a value
    fn get_self(&self, target: ActionTarget) -> Parameter<Value>;
    /// Update this data with the given value
    fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool;
}

pub trait LiveModRepr<T> {
    fn repr(&self, cur: &T) -> Namespaced<Repr>;
}

pub struct DefaultRepr;

impl<T> LiveModRepr<T> for DefaultRepr
where
    T: LiveMod,
{
    fn repr(&self, cur: &T) -> Namespaced<Repr> {
        cur.repr_default(ActionTarget::This)
    }
}

/// Slider representation for numeric values
pub struct Slider<T>(pub RangeInclusive<T>);

macro_rules! impl_slider {
    ($(($t:ty, $var:ident)),*) => {
        $(
            impl LiveModRepr<$t> for Slider<$t> {
                fn repr(&self, _cur: &$t) -> Namespaced<Repr> {
                    BuiltinRepr::$var {
                        suggested_min: (*self.0.start()).into(),
                        suggested_max: (*self.0.end()).into(),
                        storage_min: <$t>::MIN.into(),
                        storage_max: <$t>::MAX.into(),
                    }.into()
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
    fn repr(&self, _cur: &String) -> Namespaced<Repr> {
        BuiltinRepr::String { multiline: true }.into()
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
            fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
                debug_assert!(target.is_this());
                BuiltinRepr::UnsignedInteger {
                    min: $ty::MIN as u64,
                    max: $ty::MAX as u64,
                }.into()
            }

            fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool {
                debug_assert!(target.is_this());
                *self = value.try_into_unsigned_int().unwrap() as $ty;
                false
            }

            fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
                debug_assert!(target.is_this());
                Parameter::UnsignedInt(*self as u64)
            }
        }
        )*
    }
}

macro_rules! signed_primitive_impl {
    ($($ty:ident),*) => {
        $(
        impl LiveMod for $ty {
            fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
                debug_assert!(target.is_this());
                BuiltinRepr::SignedInteger {
                    min: $ty::MIN as i64,
                    max: $ty::MAX as i64,
                }.into()
            }

            fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool {
                debug_assert!(target.is_this());
                *self = value.try_into_signed_int().unwrap() as $ty;
                false
            }

            fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
                debug_assert!(target.is_this());
                Parameter::SignedInt(*self as i64)
            }
        }
        )*
    }
}

macro_rules! float_primitive_impl {
    ($($ty:ident),*) => {
        $(
        impl LiveMod for $ty {
            fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
                debug_assert!(target.is_this());
                BuiltinRepr::Float {
                    min: $ty::MIN as f64,
                    max: $ty::MAX as f64,
                }.into()
            }

            fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool {
                debug_assert!(target.is_this());
                *self = value.try_into_float().unwrap() as $ty;
                false
            }

            fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
                debug_assert!(target.is_this());
                Parameter::Float(*self as f64)
            }
        }
        )*
    }
}

unsigned_primitive_impl!(u8, u16, u32, u64, usize);
signed_primitive_impl!(i8, i16, i32, i64, isize);
float_primitive_impl!(f32, f64);

impl LiveMod for bool {
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
        debug_assert!(target.is_this());
        BuiltinRepr::Bool.into()
    }

    fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool {
        debug_assert!(target.is_this());
        *self = value.try_into_bool().unwrap();
        false
    }

    fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
        debug_assert!(target.is_this());
        Parameter::Bool(*self)
    }
}

impl LiveMod for String {
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
        debug_assert!(target.is_this());
        BuiltinRepr::String { multiline: false }.into()
    }

    fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool {
        debug_assert!(target.is_this());
        *self = value.try_into_string().unwrap();
        false
    }

    fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
        debug_assert!(target.is_this());
        Parameter::String(self.clone())
    }
}

impl<T> LiveMod for Vec<T>
where
    T: LiveMod + Default,
{
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
        if let Some((field, field_target)) = target.strip_one_field() {
            self[field.parse::<usize>().unwrap()].repr_default(field_target)
        } else {
            Namespaced {
                name: vec!["livemod".to_owned(), "vec".to_owned()],
                parameters: self
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (format!("{}", i), Parameter::Namespaced(v.repr_default(ActionTarget::This))))
                    .chain(std::iter::once(("len".to_owned(), Parameter::Namespaced(BuiltinRepr::UnsignedInteger { min: usize::MIN as u64, max: usize::MAX as u64 }.into()))))
                    .collect(),
                _marker: std::marker::PhantomData,
            }
        }
    }

    fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool {
        if let Some((field, field_target)) = target.strip_one_field() {
            if field == "len" {
                debug_assert!(field_target.is_this());
                let len = value.try_into_unsigned_int().unwrap() as usize;
                if len != self.len() {
                    self.resize_with(len, Default::default);
                    true
                } else {
                    false
                }
            } else {
                self[field.parse::<usize>().unwrap()].accept(field_target, value)
            }
        } else {
            let trigger = value.try_into_namespaced().unwrap();
            if trigger.name[2] == "rm" {
                let index = trigger.parameters["idx"].clone().try_into_unsigned_int().unwrap() as usize;
                self.remove(index);
            } else if trigger.name[2] == "swp" {
                let idx_a = trigger.parameters["a"].clone().try_into_unsigned_int().unwrap() as usize;
                let idx_b = trigger.parameters["b"].clone().try_into_unsigned_int().unwrap() as usize;
                self.swap(idx_a, idx_b);
            }
            true
        }
    }

    fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
        if let Some((field, field_target)) = target.strip_one_field() {
            self[field.parse::<usize>().unwrap()].get_self(field_target)
        } else {
            Parameter::Namespaced(Namespaced {
                name: vec!["livemod".to_owned(), "vec".to_owned()],
                parameters: self
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (format!("{}", i), v.get_self(ActionTarget::This)))
                    .collect(),
                _marker: std::marker::PhantomData,
            })
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
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
        debug_assert!(target.is_this());
        Namespaced {
            name: vec!["livemod".to_owned(), "trigger".to_owned()],
            parameters: HashMap::new(),
            _marker: std::marker::PhantomData,
        }
    }

    fn accept(&mut self, target: ActionTarget, trigger: Parameter<Value>) -> bool {
        debug_assert!(target.is_this());
        debug_assert!(trigger.try_into_namespaced().unwrap().name[2] == "trigger");
        (self.func)(&mut self.arg);
        false
    }

    fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
        debug_assert!(target.is_this());
        Parameter::Namespaced(Namespaced {
            name: vec!["livemod".to_owned(), "trigger".to_owned()],
            parameters: HashMap::new(),
            _marker: std::marker::PhantomData,
        })
    }
}
