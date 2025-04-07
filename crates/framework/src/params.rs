//! This module provides a trait for parsing path parameters.
// Adapted from https://github.com/rwf2/Rocket/blob/28891e8072136f4641a33fb8c3f2aafce9d88d5b/core/lib/src/request/from_param.rs
// See https://github.com/rwf2/Rocket/blob/28891e8072136f4641a33fb8c3f2aafce9d88d5b/LICENSE-MIT for license information
use std::str::FromStr;

/// Convert a path parameter string into a type.
///
/// ## Example
/// ```rs
/// use maudit::params::FromParam;
///
/// struct UserId(String);
///
/// impl FromParam for UserId {
///   type Error = std::io::Empty;
///
///   fn from_param(param: &str) -> Result<Self, Self::Error> {
///     Ok(UserId(param.to_string()))
///   }
/// }
/// ```
pub trait FromParam: Sized {
    /// The associated error to be returned if parsing/validation fails.
    type Error: std::fmt::Debug;

    /// Parses and validates an instance of `Self` from a path parameter string
    /// or returns an `Error` if parsing or validation fails.
    fn from_param(param: &str) -> Result<Self, Self::Error>;
}

impl FromParam for String {
    type Error = Empty;

    #[inline(always)]
    fn from_param(param: &str) -> Result<String, Self::Error> {
        Ok(param.to_string())
    }
}

macro_rules! impl_with_fromstr {
    ($($T:ty),+) => ($(
        impl FromParam for $T {
            type Error = <$T as FromStr>::Err;

            #[inline(always)]
            fn from_param(param: &str) -> Result<Self, Self::Error> {
                Ok(<$T as FromStr>::from_str(&param).unwrap())
            }
        }
    )+)
}

use std::io::Empty;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};

impl_with_fromstr! {
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64,
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize,
    NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
    bool, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr
}
