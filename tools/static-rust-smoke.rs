//! Exercise both C archives inside a Rust program that owns its runtime.

use std::ffi::{c_char, c_void, CString};

extern "C" {
    fn peios_sddl_parse_sd(out: *mut c_void, cap: usize, sddl: *const c_char) -> isize;
    fn rsi_parse_request(frame: *const c_void, len: usize, out: *mut c_void) -> i32;
}

fn main() {
    let text = CString::new("O:SYG:SYD:(A;OICI;GA;;;SY)").unwrap();
    let mut bytes = vec![0_u8; 256];
    // SAFETY: text is NUL-terminated, and bytes is writable for its stated size.
    let length =
        unsafe { peios_sddl_parse_sd(bytes.as_mut_ptr().cast(), bytes.len(), text.as_ptr()) };
    assert!(length > 0);
    // SAFETY: the documented invalid-argument path rejects null output/input;
    // no kernel operation is requested.
    assert_eq!(
        unsafe { rsi_parse_request(std::ptr::null(), 0, std::ptr::null_mut()) },
        -1
    );
    // The SDK's abort-only personality must not replace std's unwind runtime.
    #[cfg(panic = "unwind")]
    assert!(std::panic::catch_unwind(|| panic!("application unwind remains usable")).is_err());
    assert_eq!(bytes.len(), 256);
}
