
#[allow(non_camel_case_types, non_snake_case)]
pub mod bindings;

pub use self::bindings::*;

/*
use std::ffi::CString;
use std::ptr;

pub struct Context(*mut bindings::duk_context);

impl Context {
    pub fn new() -> Context {
        let context = unsafe {
            bindings::duk_create_heap(None, None, None, ptr::null_mut(), None)
        };
        Context(context)
    }

    pub fn compile_fn<S: AsRef<str>>(&self, code: S) -> Result<Compiled, ()> {
        let cstring = CString::new(code.as_ref());
        unsafe {
            bindings::duk_compile_raw(self.0,
                                      cstring.as_ptr(),
                                      code.as_ref().as_bytes().len(),
                                      bindings::DUK_COMPILE_NOFILENAME |
                                          bindings::DUK_COMPILE_FUNCTION |
                                          bindings::DUK_COMPILE_STRLEN);
            bindings::duk_get_f
        }


        )
    }

}

pub struct Compiled;

impl Drop for Context {

    fn drop(&mut self) {
        unsafe {
            bindings::duk_destroy_heap(self.0)
        }
    }
}

#[cfg(tests)]
mod tests {

    use super::*;

    #[test]
    fn test() {
        let ctx = Context::new();
        ctx.compile_fn("function (x) { return x + 1; }")
    }
}
*/