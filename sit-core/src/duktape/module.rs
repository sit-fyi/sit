use super::duk_context;

extern "C" {
    pub fn duk_module_duktape_init(ctx: *mut duk_context);
}
