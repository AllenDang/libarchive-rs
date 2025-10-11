// Simple example to verify libarchive-sys bindings work

use std::ffi::CStr;

fn main() {
    unsafe {
        // Get libarchive version
        let version_number = libarchive_sys::archive_version_number();
        let version_string = libarchive_sys::archive_version_string();
        let version_details = libarchive_sys::archive_version_details();

        println!("libarchive version number: {}", version_number);
        println!(
            "libarchive version string: {}",
            CStr::from_ptr(version_string).to_string_lossy()
        );
        println!(
            "libarchive version details: {}",
            CStr::from_ptr(version_details).to_string_lossy()
        );
    }
}
