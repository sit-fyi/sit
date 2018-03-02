#[allow(non_camel_case_types, non_snake_case)]
pub mod bindings;
pub use self::bindings::*;

#[cfg(feature = "duktape-require")]
#[allow(non_camel_case_types, non_snake_case)]
pub mod module;
#[cfg(feature = "duktape-require")]
pub use self::module::*;
