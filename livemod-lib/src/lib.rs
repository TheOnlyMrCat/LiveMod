//! # livemod - Runtime modification of program parameters

use std::array::IntoIter;
use std::error::Error;
use std::fmt::Display;
use std::hash::Hash;
use std::iter::FromIterator;
use std::ops::RangeInclusive;
use std::string::FromUtf8Error;

pub use hashlink;
use hashlink::LinkedHashMap;

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
    /// A strin
    ///
    /// Maps to `livemod:string`
    String { multiline: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeserializeError {
    UnexpectedEOF,
    UnexpectedTerminator { previous: String },
    InvalidParameter(u8),
    NonUTF8(FromUtf8Error),
}

impl From<FromUtf8Error> for DeserializeError {
    fn from(v: FromUtf8Error) -> Self {
        Self::NonUTF8(v)
    }
}

impl Display for DeserializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeserializeError::UnexpectedEOF => write!(f, "Unexpected end-of-file"),
            DeserializeError::UnexpectedTerminator { previous } => write!(f, "Unexpected terminator in middle of {}", previous),
            DeserializeError::InvalidParameter(b) => write!(f, "Invalid parameter type: {}", *b as char),
            DeserializeError::NonUTF8(_) => write!(f, "Expected UTF-8"),
        }
    }
}

impl Error for DeserializeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DeserializeError::NonUTF8(e) => Some(e),
            _ => None
        }
    }
}

/// Marker type to specify a representation parameter
#[derive(Clone, Copy, Debug)]
pub struct Repr;

/// Marker type to specify a value parameter
#[derive(Clone, Copy, Debug)]
pub struct Value;

/// A value in the LiveMod message transfer system
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
            Parameter::SignedInt(i) => format!("i{}", i),
            Parameter::UnsignedInt(i) => format!("u{}", i),
            Parameter::Float(f) => format!("d{}", f),
            Parameter::Bool(true) => "t".to_owned(),
            Parameter::Bool(false) => "f".to_owned(),
            Parameter::String(s) => format!("s{}-{}", s.as_bytes().len(), s),
            Parameter::Namespaced(n) => format!("n{}", n.serialize()),
        }
    }

    pub fn deserialize(mut s: &mut dyn Iterator<Item = u8>) -> Result<Parameter<T>, DeserializeError> {
        Ok(match s.next().unwrap() {
            // Terminating ';' consumed by take_while
            b'i' => Parameter::SignedInt(s.take_while(|b| b.is_ascii_digit() || *b == b'-').map(|b| b as char).collect::<String>().parse().unwrap()),
            b'd' => Parameter::Float(s.take_while(|b| b.is_ascii_digit() || *b == b'-' || *b == b'.').map(|b| b as char).collect::<String>().parse().unwrap()),
            b'u' => Parameter::UnsignedInt(s.take_while(|b| b.is_ascii_digit() || *b == b'-').map(|b| b as char).collect::<String>().parse().unwrap()),
            b't' => {
                s.next(); // consume the terminating `;`
                Parameter::Bool(true)
            },
            b'f' => {
                s.next(); // consume the terminating `;`
                Parameter::Bool(false)
            },
            b's' => {
                let len = (&mut s).take_while(|b| b.is_ascii_digit()).map(|b| b as char).collect::<String>().parse().unwrap();
                // take_while will have consumed the separator `-`
                let string = String::from_utf8(s.take(len).collect())?;
                s.next(); // consume the terminating `;`
                Parameter::String(string)
            }
            b'n' => {
                let namespaced = Namespaced::deserialize(s)?;
                s.next(); // consume the terminating `;`
                Parameter::Namespaced(namespaced)
            },
            b => return Err(DeserializeError::InvalidParameter(b)),
        })
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

    pub fn as_string(&self) -> Option<&String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_namespaced(&self) -> Option<&Namespaced<T>> {
        if let Self::Namespaced(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// A namespaced value in the LiveMod message transfer system
///
/// This consists of a namespace, a name, and a set of labelled parameters encoding information for the type.
/// Namespaces should start with the crate name which defines the type, and all parts of a namespaced name
/// must only contain characters valid in a rust crate name ([A-Za-z_\-])
#[derive(Clone, Debug)]
pub struct Namespaced<T> {
    pub name: Vec<String>,
    pub parameters: LinkedHashMap<String, Parameter<T>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Namespaced<T> {
    pub fn new(name: Vec<String>, parameters: LinkedHashMap<String, Parameter<T>>) -> Self {
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
            s.push(';');
        }
        s.push('}');
        s
    }

    pub fn deserialize(mut s: &mut dyn Iterator<Item = u8>) -> Result<Namespaced<T>, DeserializeError> {
        let name = {
            let mut name = Vec::new();
            loop {
                let b = s.next();
                match b {
                    Some(b'{') => break,
                    Some(b) => name.push(b),
                    None => return Err(DeserializeError::UnexpectedEOF),
                }
            }
            //TODO: Name could allow internal colons?
            String::from_utf8(name)?.split(':').map(|s| s.trim().to_owned()).collect()
        };

        let mut parameters = LinkedHashMap::new();
        loop {
            let key = {
                let mut key = match s.next() {
                    Some(b'}') => break,
                    Some(b) => {
                        let mut key = Vec::new();
                        key.push(b);
                        key
                    },
                    None => return Err(DeserializeError::UnexpectedEOF),
                };
                loop {
                    let b = s.next();
                    match b {
                        Some(b'}') => return Err(DeserializeError::UnexpectedTerminator { previous: String::from_utf8_lossy(&key).into_owned() }),
                        Some(b'=') => break,
                        Some(b) => key.push(b),
                        None => return Err(DeserializeError::UnexpectedEOF),
                    }
                }
                String::from_utf8(key)?
            };

            let parameter = Parameter::deserialize(&mut s)?;
            parameters.insert(key, parameter);
        }

        Ok(Namespaced {
            name,
            parameters,
            _marker: std::marker::PhantomData,
        })
    }
}

impl Namespaced<Repr> {
    pub fn basic_structure_repr(
        name: &str,
        fields: &[(String, Namespaced<Repr>)],
    ) -> Namespaced<Repr> {
        Namespaced {
            name: vec!["livemod".to_owned(), "struct".to_owned()],
            parameters: LinkedHashMap::from_iter(IntoIter::new([
                ("name".to_owned(), Parameter::String(name.to_owned())),
                (
                    "fields".to_owned(),
                    Parameter::Namespaced(Namespaced {
                        name: vec!["livemod".to_owned(), "fields".to_owned()],
                        parameters: LinkedHashMap::from_iter(fields.iter().map(|(name, field)| {
                            (name.to_owned(), Parameter::Namespaced(field.clone()))
                        })),
                        _marker: std::marker::PhantomData,
                    }),
                ),
            ])),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn fields_repr(fields: &[(String, Parameter<Repr>)]) -> Namespaced<Repr> {
        Namespaced {
            name: vec!["livemod".to_owned(), "fields".to_owned()],
            parameters: LinkedHashMap::from_iter(
                fields
                    .iter()
                    .map(|(name, field)| (name.to_owned(), field.clone())),
            ),
            _marker: std::marker::PhantomData,
        }
    }
}

impl Namespaced<Value> {
    pub fn basic_structure_value(fields: &[(String, Parameter<Value>)]) -> Namespaced<Value> {
        Namespaced {
            name: vec!["livemod".to_owned(), "struct".to_owned()],
            parameters: LinkedHashMap::from_iter(
                fields
                    .iter()
                    .map(|(name, field)| (name.to_owned(), field.clone())),
            ),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn fields_value(fields: &[(String, Parameter<Value>)]) -> Namespaced<Value> {
        Namespaced {
            name: vec!["livemod".to_owned(), "fields".to_owned()],
            parameters: LinkedHashMap::from_iter(
                fields
                    .iter()
                    .map(|(name, field)| (name.to_owned(), field.clone())),
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
                parameters: LinkedHashMap::new(),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::String { multiline } => Namespaced {
                name: vec!["livemod".to_owned(), "string".to_owned()],
                parameters: LinkedHashMap::from_iter(IntoIter::new([(
                    "multiline".to_owned(),
                    Parameter::Bool(multiline),
                )])),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::SignedInteger { min, max } => Namespaced {
                name: vec!["livemod".to_owned(), "sint".to_owned()],
                parameters: LinkedHashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::SignedInt(min)),
                    ("max".to_owned(), Parameter::SignedInt(max)),
                ])),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::UnsignedInteger { min, max } => Namespaced {
                name: vec!["livemod".to_owned(), "uint".to_owned()],
                parameters: LinkedHashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::UnsignedInt(min)),
                    ("max".to_owned(), Parameter::UnsignedInt(max)),
                ])),
                _marker: std::marker::PhantomData,
            },
            BuiltinRepr::Float { min, max } => Namespaced {
                name: vec!["livemod".to_owned(), "float".to_owned()],
                parameters: LinkedHashMap::from_iter(IntoIter::new([
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
                parameters: LinkedHashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::SignedInt(storage_min)),
                    ("max".to_owned(), Parameter::SignedInt(storage_max)),
                    (
                        "suggested_min".to_owned(),
                        Parameter::SignedInt(suggested_min),
                    ),
                    (
                        "suggested_max".to_owned(),
                        Parameter::SignedInt(suggested_max),
                    ),
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
                parameters: LinkedHashMap::from_iter(IntoIter::new([
                    ("min".to_owned(), Parameter::UnsignedInt(storage_min)),
                    ("max".to_owned(), Parameter::UnsignedInt(storage_max)),
                    (
                        "suggested_min".to_owned(),
                        Parameter::UnsignedInt(suggested_min),
                    ),
                    (
                        "suggested_max".to_owned(),
                        Parameter::UnsignedInt(suggested_max),
                    ),
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
                parameters: LinkedHashMap::from_iter(IntoIter::new([
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

/// Data which can be modified by the LiveMod API
pub trait LiveMod: Send {
    /// The default representation of the data.
    ///
    /// The representation of the data may be dependent on the current value of `self`.
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr>;

    /// Get the current value of `self` in the LiveMod message format.
    fn get_self(&self, target: ActionTarget) -> Parameter<Value>;

    /// Update this data with the given value
    fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool;
}

/// Data which provides extra guarantees about its representation.
///
/// Types implementing `LiveModCtor` must have a sane representation which doesn't depend on its current value. Additionally,
/// It must be possible to construct it directly from the LiveMod message format.
pub trait LiveModCtor: LiveMod {
    fn repr_static() -> Namespaced<Repr>;
    fn from_value(value: Parameter<Value>) -> Option<Self>
    where
        Self: Sized;
}

/// Provider of an alternate representation for a LiveMod type
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
                Self::repr_static()
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

        impl LiveModCtor for $ty {
            fn repr_static() -> Namespaced<Repr> {
                BuiltinRepr::UnsignedInteger {
                    min: $ty::MIN as u64,
                    max: $ty::MAX as u64,
                }.into()
            }

            fn from_value(value: Parameter<Value>) -> Option<Self> {
                value.try_into_unsigned_int().ok().map(|v| v as $ty)
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
                Self::repr_static()
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

        impl LiveModCtor for $ty {
            fn repr_static() -> Namespaced<Repr> {
                BuiltinRepr::SignedInteger {
                    min: $ty::MIN as i64,
                    max: $ty::MAX as i64,
                }.into()
            }

            fn from_value(value: Parameter<Value>) -> Option<Self> {
                value.try_into_signed_int().ok().map(|v| v as $ty)
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
                Self::repr_static()
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

        impl LiveModCtor for $ty {
            fn repr_static() -> Namespaced<Repr> {
                BuiltinRepr::Float {
                    min: $ty::MIN as f64,
                    max: $ty::MAX as f64,
                }.into()
            }

            fn from_value(value: Parameter<Value>) -> Option<Self> {
                value.try_into_float().ok().map(|v| v as $ty)
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
        Self::repr_static()
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

impl LiveModCtor for bool {
    fn repr_static() -> Namespaced<Repr> {
        BuiltinRepr::Bool.into()
    }

    fn from_value(value: Parameter<Value>) -> Option<Self> {
        value.try_into_bool().ok()
    }
}

impl LiveMod for String {
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
        debug_assert!(target.is_this());
        Self::repr_static()
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

impl LiveModCtor for String {
    fn repr_static() -> Namespaced<Repr> {
        BuiltinRepr::String { multiline: false }.into()
    }

    fn from_value(value: Parameter<Value>) -> Option<Self> {
        value.try_into_string().ok()
    }
}

impl<T> LiveMod for Box<T>
where
    T: LiveMod,
{
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
        (**self).repr_default(target)
    }

    fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool {
        (**self).accept(target, value)
    }

    fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
        (**self).get_self(target)
    }
}

impl<T> LiveModCtor for Box<T>
where
    T: LiveModCtor,
{
    fn repr_static() -> Namespaced<Repr> {
        T::repr_static()
    }

    fn from_value(value: Parameter<Value>) -> Option<Self> {
        T::from_value(value).map(Box::new)
    }
}

impl LiveMod for Box<dyn LiveMod> {
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
        (**self).repr_default(target)
    }

    fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool {
        (**self).accept(target, value)
    }

    fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
        (**self).get_self(target)
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
                    .map(|(i, v)| {
                        (
                            format!("{}", i),
                            Parameter::Namespaced(v.repr_default(ActionTarget::This)),
                        )
                    })
                    .chain(std::iter::once((
                        "len".to_owned(),
                        Parameter::UnsignedInt(self.len() as u64),
                    )))
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
                let index = trigger.parameters["idx"]
                    .clone()
                    .try_into_unsigned_int()
                    .unwrap() as usize;
                self.remove(index);
            } else if trigger.name[2] == "swp" {
                let idx_a = trigger.parameters["a"]
                    .clone()
                    .try_into_unsigned_int()
                    .unwrap() as usize;
                let idx_b = trigger.parameters["b"]
                    .clone()
                    .try_into_unsigned_int()
                    .unwrap() as usize;
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

impl<K, V> LiveMod for std::collections::HashMap<K, V>
where
    K: LiveModCtor + Eq + Hash + std::fmt::Debug,
    V: LiveMod + Default,
{
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
        if let Some((field, field_target)) = target.strip_one_field() {
            // Field will be "keys" or "values"
            match field {
                "keys" => {
                    // Repr of a key will always be the same
                    K::repr_static()
                }
                "values" => {
                    if let Some((field, field_target)) = field_target.strip_one_field() {
                        self.get(&K::from_value(Parameter::deserialize(&mut field.bytes()).unwrap()).unwrap())
                            .unwrap()
                            .repr_default(field_target)
                    } else {
                        unimplemented!()
                    }
                }
                _ => unimplemented!(),
            }
        } else {
            Namespaced {
                name: vec!["livemod".to_owned(), "map".to_owned()],
                parameters: IntoIter::new([
                    ("key".to_owned(), Parameter::Namespaced(K::repr_static())),
                    (
                        "keys".to_owned(),
                        Parameter::Namespaced(Namespaced {
                            name: vec!["livemod".to_owned(), "fields".to_owned()],
                            parameters: self
                                .iter()
                                .map(|(k, _v)| {
                                    (
                                        k.get_self(ActionTarget::This).serialize(),
                                        Parameter::Namespaced(k.repr_default(ActionTarget::This)),
                                    )
                                })
                                .collect(),
                            _marker: std::marker::PhantomData,
                        }),
                    ),
                    (
                        "values".to_owned(),
                        Parameter::Namespaced(Namespaced {
                            name: vec!["livemod".to_owned(), "fields".to_owned()],
                            parameters: self
                                .iter()
                                .map(|(k, v)| {
                                    (
                                        k.get_self(ActionTarget::This).serialize(),
                                        Parameter::Namespaced(v.repr_default(ActionTarget::This)),
                                    )
                                })
                                .collect(),
                            _marker: std::marker::PhantomData,
                        }),
                    ),
                ])
                .collect(),
                _marker: std::marker::PhantomData,
            }
        }
    }

    fn accept(&mut self, target: ActionTarget, value: Parameter<Value>) -> bool {
        if let Some((field, field_target)) = target.strip_one_field() {
            // As with repr_default, will be "keys" or "values"
            match field {
                "keys" => {
                    if let Some((field, field_target)) = field_target.strip_one_field() {
                        let (mut k, v) = self
                            .remove_entry(
                                &K::from_value(Parameter::deserialize(&mut field.bytes()).unwrap()).unwrap(),
                            )
                            .unwrap();
                        k.accept(field_target, value);
                        self.insert(k, v);
                        true
                    } else {
                        unimplemented!()
                    }
                }
                "values" => {
                    if let Some((field, field_target)) = field_target.strip_one_field() {
                        self.get_mut(
                            &K::from_value(Parameter::deserialize(&mut field.bytes()).unwrap()).unwrap(),
                        )
                        .unwrap()
                        .accept(field_target, value)
                    } else {
                        unimplemented!()
                    }
                }
                _ => unimplemented!(),
            }
        } else {
            let trigger = value.try_into_namespaced().unwrap();
            if trigger.name[2] == "rm" {
                let key = K::from_value(trigger.parameters["key"].clone()).unwrap();
                self.remove(&key);
            } else if trigger.name[2] == "insert" {
                let key = K::from_value(trigger.parameters["key"].clone()).unwrap();
                self.insert(key, Default::default());
            }
            true
        }
    }

    fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
        if let Some((field, field_target)) = target.strip_one_field() {
            self.get(&K::from_value(Parameter::deserialize(&mut field.bytes()).unwrap()).unwrap())
                .unwrap()
                .get_self(field_target)
        } else {
            Parameter::Namespaced(Namespaced {
                name: vec!["livemod".to_owned(), "map".to_owned()],
                parameters: IntoIter::new([
                    (
                        "keys".to_owned(),
                        Parameter::Namespaced(Namespaced {
                            name: vec!["livemod".to_owned(), "fields".to_owned()],
                            parameters: self
                                .iter()
                                .map(|(k, _v)| {
                                    let val = k.get_self(ActionTarget::This);
                                    (val.serialize(), val)
                                })
                                .collect(),
                            _marker: std::marker::PhantomData,
                        }),
                    ),
                    (
                        "values".to_owned(),
                        Parameter::Namespaced(Namespaced {
                            name: vec!["livemod".to_owned(), "fields".to_owned()],
                            parameters: self
                                .iter()
                                .map(|(k, v)| {
                                    (
                                        k.get_self(ActionTarget::This).serialize(),
                                        v.get_self(ActionTarget::This),
                                    )
                                })
                                .collect(),
                            _marker: std::marker::PhantomData,
                        }),
                    ),
                ])
                .collect(),
                _marker: std::marker::PhantomData,
            })
        }
    }
}

pub struct TriggerFn<A: Send, F: FnMut(&mut A) + Send> {
    arg: A,
    func: F,
}

impl<A: Send, F: FnMut(&mut A) + Send> TriggerFn<A, F> {
    pub fn new(arg: A, func: F) -> TriggerFn<A, F> {
        TriggerFn { arg, func }
    }
}

impl<A: Send, F: FnMut(&mut A) + Send> LiveMod for TriggerFn<A, F> {
    fn repr_default(&self, target: ActionTarget) -> Namespaced<Repr> {
        debug_assert!(target.is_this());
        Namespaced {
            name: vec!["livemod".to_owned(), "trigger".to_owned()],
            parameters: LinkedHashMap::new(),
            _marker: std::marker::PhantomData,
        }
    }

    fn accept(&mut self, target: ActionTarget, trigger: Parameter<Value>) -> bool {
        debug_assert!(target.is_this());
        debug_assert!(trigger.try_into_namespaced().unwrap().name[1] == "trigger");
        (self.func)(&mut self.arg);
        false
    }

    fn get_self(&self, target: ActionTarget) -> Parameter<Value> {
        debug_assert!(target.is_this());
        Parameter::Namespaced(Namespaced {
            name: vec!["livemod".to_owned(), "trigger".to_owned()],
            parameters: LinkedHashMap::new(),
            _marker: std::marker::PhantomData,
        })
    }
}
