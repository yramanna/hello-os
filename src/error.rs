//! Error handling.

pub type Result<T> = core::result::Result<T, Error>;

use displaydoc::Display;

/// An error.
#[non_exhaustive]
#[derive(Clone, Debug, Display)]
pub enum Error {
    /// No such script is defined.
    NoSuchScript,


    /// Invalid descriptor type: {0}
    InvalidDescriptorType(u8),

    /// Other error.
    Other(&'static str),
}
