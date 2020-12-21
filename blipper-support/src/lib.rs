#![cfg_attr(not(feature = "utils"), no_std)]

#[cfg(feature = "utils")]
pub mod decoder;
#[cfg(feature = "utils")]
pub mod link;

pub mod protocol;

#[cfg(feature = "utils")]
pub use decoder::Decoders;
#[cfg(feature = "utils")]
pub use link::SerialLink;
