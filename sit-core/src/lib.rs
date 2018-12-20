//! sit-core is a library that implements SIT (SIT's an Issue Tracker)
//!
//! It is used by `sit` tooling and can be used by other projects to
//! build scripts or additional tooling for SIT.
//!
//! The main entry point to this library is [`Repository`] structure.
//!
//! [`Repository`]: repository/struct.Repository.html

pub mod path;
pub mod hash;
pub mod encoding;
#[cfg(feature = "deprecated-item-api")]
pub mod id;
pub mod repository;
#[cfg(feature = "deprecated-item-api")]
pub mod item;
#[cfg(feature = "deprecated-item-api")]
pub use crate::item::Item;
pub mod record;
pub use crate::record::Record;
pub use crate::repository::{Repository, Error as RepositoryError};
pub mod reducers;
pub use crate::reducers::Reducer;
#[cfg(feature = "duktape")]
pub mod duktape;
