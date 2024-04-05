use std::ffi::CStr;
use libvips::bindings::{vips_error_buffer, vips_error_clear};

pub fn get_error_message() -> String {
    unsafe {
        let error_buffer_ptr: *const ::std::os::raw::c_char = vips_error_buffer();
        let c_str = CStr::from_ptr(error_buffer_ptr);
        let error_message = c_str.to_string_lossy().into_owned();
        vips_error_clear();
        format!("{:?}", error_message)
    }
}
