use std::ffi::CString;
use std::ptr;

pub fn get_memory_level() -> Option<u32> {
    let name = CString::new("kern.memorystatus_level").unwrap();
    let mut level: libc::c_int = 0;
    let mut size = std::mem::size_of::<libc::c_int>();

    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            &mut level as *mut _ as *mut libc::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if result == 0 {
        Some(level as u32)
    } else {
        None
    }
}
