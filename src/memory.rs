use std::ffi::CString;
use std::ptr;

/// Fetches the current memory status level from the XNU kernel using `sysctlbyname`.
///
/// This function directly queries `kern.memorystatus_level` which provides an
/// integer representing the system's memory pressure. Lower values indicate
/// higher memory pressure (e.g., 0 for critical, 100 for plenty of free memory).
///
/// # Returns
///
/// An `Option<u32>`:
/// - `Some(level)` if the `sysctlbyname` call is successful, where `level` is
///   the memory status as a `u32`.
/// - `None` if the `sysctlbyname` call fails (e.g., the sysctl name is invalid
///   or there's an error retrieving the value).
pub fn get_memory_level() -> Option<u32> {
    // Convert the sysctl name string to a CString, required for FFI.
    let name = CString::new("kern.memorystatus_level").unwrap();
    // Initialize a variable to hold the memory level. It's a C integer.
    let mut level: libc::c_int = 0;
    // Get the size of the c_int, which sysctlbyname needs to know how much memory to write.
    let mut size = std::mem::size_of::<libc::c_int>();

    // Call the unsafe libc::sysctlbyname function.
    // This is an FFI (Foreign Function Interface) call to a C function.
    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),                             // Pointer to the sysctl name
            &mut level as *mut _ as *mut libc::c_void, // Pointer to where the result should be stored
            &mut size,                                 // Pointer to the size of the result buffer
            ptr::null_mut(),                           // No new value to set, so null
            0,                                         // Size of new value, 0 as we're only reading
        )
    };

    // If result is 0, the call was successful.
    if result == 0 {
        Some(level as u32) // Convert the C integer to a u32 and return.
    } else {
        None // If not successful, return None.
    }
}
