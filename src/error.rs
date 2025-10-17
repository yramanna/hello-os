//! Error handling.
pub type Result<T> = core::result::Result<T, Error>;

/// An error.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum Error {
    /// No such script is defined.
    NoSuchScript,

    /// Invalid descriptor type: {0}
    InvalidDescriptorType(u8),

    /// Other error.
    Other(&'static str),
}
