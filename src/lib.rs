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
//! use libarchive2::ReadArchive;
//!
//! // Reading from a file (lifetime is 'static)
//! let mut archive = ReadArchive::open("archive.tar.gz")?;
//!
//! while let Some(entry) = archive.next_entry()? {
//!     println!("File: {}", entry.pathname().unwrap_or_default());
//!     // Read entry data...
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Writing an archive
//!
//! ```no_run
//! use libarchive2::{WriteArchive, ArchiveFormat, CompressionFormat};
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

mod acl_xattr;
mod callbacks;
mod entry;
mod error;
mod extract;
mod format;
mod locale;
mod match_filter;
mod read_disk;
mod reader;
mod writer;

pub use acl_xattr::{
    AclEntry, AclPermissions, AclTag, AclType, EntryAclExt, EntryMutAclExt, Xattr,
};
pub use callbacks::{CallbackReader, CallbackWriter, ProgressCallback, ProgressTracker};
pub use entry::{Entry, EntryMut, FileType};
pub use error::{Error, Result};
pub use extract::{ExtractFlags, WriteDisk};
pub use format::{
    ArchiveFormat, CompressionFormat, CompressionLevel, FilterOption, FormatOption, ReadFormat,
    ZipCompressionMethod,
};
pub use match_filter::ArchiveMatch;
pub use read_disk::{ReadDisk, ReadDiskFlags, SymlinkMode};
pub use reader::ReadArchive;
pub use writer::WriteArchive;

/// Returns the version string of the underlying libarchive library
pub fn version() -> String {
    // SAFETY: archive_version_string returns a static string that is always valid
    unsafe {
        let ptr = libarchive2_sys::archive_version_string();
        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Returns the version number of the underlying libarchive library
pub fn version_number() -> i32 {
    // SAFETY: Simple integer return, no pointers involved
    unsafe { libarchive2_sys::archive_version_number() }
}

/// Returns detailed version information including linked libraries
pub fn version_details() -> String {
    // SAFETY: archive_version_details returns a static string that is always valid
    unsafe {
        let ptr = libarchive2_sys::archive_version_details();
        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}
