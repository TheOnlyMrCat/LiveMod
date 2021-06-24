//! # livemod

#[cfg(feature = "livemod-derive")]
pub use livemod_derive::LiveMod;

mod enabled;
mod disabled;

#[cfg(not(feature = "disabled"))]
pub use enabled::*;

#[cfg(feature = "disabled")]
pub use disabled::*;

use nanoserde::{DeBin, SerBin};

#[derive(SerBin, DeBin)]
pub struct StructData {
    pub name: String,
    pub data_type: StructDataType,
}

#[derive(SerBin, DeBin)]
pub enum StructDataType {
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
    SignedInteger {
        min: i64,
        max: i64,
    },
    UnsignedInteger {
        min: u64,
        max: u64,
    },
    Struct {
        name: String,
        fields: Vec<StructData>,
    },
}

#[derive(SerBin, DeBin)]
pub enum StructDataValue {
    SignedInt(i64),
    UnsignedInt(u64),
}

impl StructDataValue {
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
}

pub trait LiveMod {
    fn data_type(&self) -> StructDataType;
    fn get_named_value(&mut self, name: &str) -> &mut dyn LiveMod;
    fn set_self(&mut self, value: StructDataValue);
}

macro_rules! unsigned_primitive_impl {
    ($($ty:ident),*) => {
        $(
        impl LiveMod for $ty {
            fn data_type(&self) -> StructDataType {
                StructDataType::UnsignedInteger {
                    min: $ty::MIN as u64,
                    max: $ty::MAX as u64,
                }
            }

            fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
                unimplemented!()
            }

            fn set_self(&mut self, value: StructDataValue) {
                match value {
                    StructDataValue::UnsignedInt(val) => *self = val as $ty,
                    _ => unimplemented!(),
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
            fn data_type(&self) -> StructDataType {
                StructDataType::SignedInteger {
                    min: $ty::MIN as i64,
                    max: $ty::MAX as i64,
                }
            }

            fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
                unimplemented!()
            }

            fn set_self(&mut self, value: StructDataValue) {
                match value {
                    StructDataValue::SignedInt(val) => *self = val as $ty,
                    _ => unimplemented!(),
                }
            }
        }
        )*
    }
}

unsigned_primitive_impl!(u8, u16, u32, u64, usize);
signed_primitive_impl!(i8, i16, i32, i64, isize);