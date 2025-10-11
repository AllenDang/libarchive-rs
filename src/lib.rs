//! Safe Rust bindings for libarchive
//!
//! This crate provides idiomatic Rust bindings to libarchive, supporting reading and writing
//! various archive formats (tar, zip, 7z, etc.) with multiple compression formats.
//!
//! # Examples
//!
//! ## Reading an archive
//!
//! ```no_run
//! use libarchive::ReadArchive;
//!
//! let mut archive = ReadArchive::open("archive.tar.gz")?;
//!
//! while let Some(entry) = archive.next_entry()? {
//!     println!("File: {}", entry.pathname().unwrap_or(""));
//!     // Read entry data...
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Writing an archive
//!
//! ```no_run
//! use libarchive::{WriteArchive, ArchiveFormat, CompressionFormat};
//!
//! let mut archive = WriteArchive::new()
//!     .format(ArchiveFormat::Tar)
//!     .compression(CompressionFormat::Gzip)
//!     .open_file("output.tar.gz")?;
//!
//! archive.add_file("file.txt", b"Hello, world!")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![deny(missing_docs)]

mod entry;
mod error;
mod format;
mod reader;
mod writer;

pub use entry::{Entry, EntryMut, FileType};
pub use error::{Error, Result};
pub use format::{ArchiveFormat, CompressionFormat, ReadFormat};
pub use reader::ReadArchive;
pub use writer::WriteArchive;

/// Returns the version string of the underlying libarchive library
pub fn version() -> String {
    unsafe {
        let ptr = libarchive2_sys::archive_version_string();
        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Returns the version number of the underlying libarchive library
pub fn version_number() -> i32 {
    unsafe { libarchive2_sys::archive_version_number() }
}

/// Returns detailed version information including linked libraries
pub fn version_details() -> String {
    unsafe {
        let ptr = libarchive2_sys::archive_version_details();
        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}
