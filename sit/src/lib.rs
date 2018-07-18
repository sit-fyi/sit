extern crate sit_core;
extern crate which;
#[macro_use] extern crate derive_error;
mod cli;
mod module_iter;

pub use module_iter::{ScriptModuleIterator, ScriptModule};
