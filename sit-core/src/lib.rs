//! sit-core is a library that implements SIT (SIT's an Issue Tracker)
//!
//! It is used by `sit` tooling and can be used by other projects to
//! build scripts or additional tooling for SIT.
//!
//! The main entry point to this library is [`Repository`] structure.
//!
//! [`Repository`]: repository/struct.Repository.html

#[macro_use] extern crate derive_error;
#[macro_use] extern crate typed_builder;

// Serialization
extern crate serde;
#[macro_use] extern crate serde_derive;
pub extern crate serde_json;

extern crate tempdir;
extern crate glob;
extern crate data_encoding;
#[macro_use] extern crate lazy_static;
extern crate tini;
extern crate fs2;

// Hashing
extern crate digest;
#[cfg(feature = "blake2")] extern crate blake2;
#[cfg(feature = "sha-1")] extern crate sha1;

#[cfg(feature = "uuid")] extern crate uuid;

#[cfg(feature = "memmap")] extern crate memmap;

// Crates necessary for testing
#[cfg(test)] #[macro_use] extern crate assert_matches;


pub mod hash;
pub mod encoding;
pub mod id;
pub mod repository;
pub mod issue;
pub use issue::Issue;
pub mod record;
pub use record::Record;
pub use repository::{Repository, Error as RepositoryError};
pub mod reducers;
pub use reducers::Reducer;
#[cfg(feature = "duktape")]
pub mod duktape;
pub mod cfg;
pub mod lock;
pub use lock::{FileLock, Lock};
