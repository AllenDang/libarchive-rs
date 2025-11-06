//! Archive extraction functionality

use crate::entry::EntryMut;
use crate::error::{Error, Result};
use std::ops::{BitOr, BitOrAssign};

/// Flags for controlling extraction behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractFlags(i32);

impl ExtractFlags {
    /// No special extraction flags
    pub const NONE: ExtractFlags = ExtractFlags(0);

    /// Set owner/group on extracted files
    pub const OWNER: ExtractFlags = ExtractFlags(0x0001);

    /// Restore file permissions
    pub const PERM: ExtractFlags = ExtractFlags(0x0002);

    /// Restore modification time
    pub const TIME: ExtractFlags = ExtractFlags(0x0004);

    /// Don't overwrite existing files
    pub const NO_OVERWRITE: ExtractFlags = ExtractFlags(0x0008);

    /// Unlink file before creating
    pub const UNLINK: ExtractFlags = ExtractFlags(0x0010);

    /// Restore ACLs (Access Control Lists)
    pub const ACL: ExtractFlags = ExtractFlags(0x0020);

    /// Restore file flags (e.g., immutable, append-only)
    pub const FFLAGS: ExtractFlags = ExtractFlags(0x0040);

    /// Restore extended attributes
    pub const XATTR: ExtractFlags = ExtractFlags(0x0080);

    /// Guard against symlink attacks
    pub const SECURE_SYMLINKS: ExtractFlags = ExtractFlags(0x0100);

    /// Reject entries with '..' in path
    pub const SECURE_NODOTDOT: ExtractFlags = ExtractFlags(0x0200);

    /// Don't create parent directories automatically
    pub const NO_AUTODIR: ExtractFlags = ExtractFlags(0x0400);

    /// Don't overwrite newer files
    pub const NO_OVERWRITE_NEWER: ExtractFlags = ExtractFlags(0x0800);

    /// Write sparse files with holes
    pub const SPARSE: ExtractFlags = ExtractFlags(0x1000);

    /// Restore Mac OS metadata
    pub const MAC_METADATA: ExtractFlags = ExtractFlags(0x2000);

    /// Don't use HFS+ compression
    pub const NO_HFS_COMPRESSION: ExtractFlags = ExtractFlags(0x4000);

    /// Force HFS+ compression
    pub const HFS_COMPRESSION_FORCED: ExtractFlags = ExtractFlags(0x8000);

    /// Reject absolute paths
    pub const SECURE_NOABSOLUTEPATHS: ExtractFlags = ExtractFlags(0x10000);

    /// Clear no-change flags when unlinking
    pub const CLEAR_NOCHANGE_FFLAGS: ExtractFlags = ExtractFlags(0x20000);

    /// Use safe writes (rename after extraction)
    pub const SAFE_WRITES: ExtractFlags = ExtractFlags(0x40000);

    /// Get the raw integer value of the flags
    pub fn bits(&self) -> i32 {
        self.0
    }
}

impl BitOr for ExtractFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        ExtractFlags(self.0 | rhs.0)
    }
}

impl BitOrAssign for ExtractFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Archive writer for extracting entries to disk
///
/// This provides the `archive_write_disk` API for writing archive entries
/// directly to the filesystem.
///
/// # Thread Safety
///
/// `WriteDisk` is `Send` but not `Sync`. You can transfer ownership between threads,
/// but cannot share references across threads.
pub struct WriteDisk {
    archive: *mut libarchive2_sys::archive,
}

// SAFETY: WriteDisk can be sent between threads because the archive pointer
// is owned exclusively by this instance and libarchive write_disk objects
// can be used from different threads (just not concurrently).
unsafe impl Send for WriteDisk {}

// Note: WriteDisk is NOT Sync because libarchive archives are not thread-safe
// for concurrent access.

impl WriteDisk {
    /// Create a new disk writer
    pub fn new() -> Result<Self> {
        unsafe {
            let archive = libarchive2_sys::archive_write_disk_new();
            if archive.is_null() {
                return Err(Error::NullPointer);
            }
            Ok(WriteDisk { archive })
        }
    }

    /// Set extraction options
    pub fn set_options(&mut self, flags: ExtractFlags) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_write_disk_set_options(self.archive, flags.bits()),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Use standard lookup functions for user/group names
    ///
    /// This enables looking up uid/gid from uname/gname using system calls
    pub fn set_standard_lookup(&mut self) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_write_disk_set_standard_lookup(self.archive),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Write an entry header to disk
    ///
    /// This creates the file/directory/etc on disk
    pub fn write_header(&mut self, entry: &EntryMut) -> Result<()> {
        // Set locale to UTF-8 on Windows to handle non-ASCII filenames correctly
        let _guard = crate::locale::WindowsUTF8LocaleGuard::new();

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_write_header(self.archive, entry.entry),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Write data for the current entry
    pub fn write_data(&mut self, data: &[u8]) -> Result<usize> {
        unsafe {
            let ret = libarchive2_sys::archive_write_data(
                self.archive,
                data.as_ptr() as *const std::os::raw::c_void,
                data.len(),
            );

            if ret < 0 {
                Err(Error::from_archive(self.archive))
            } else {
                Ok(ret as usize)
            }
        }
    }

    /// Finish writing the current entry
    pub fn finish_entry(&mut self) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_write_finish_entry(self.archive),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Close and free the disk writer
    pub fn close(mut self) -> Result<()> {
        unsafe {
            if !self.archive.is_null() {
                Error::from_return_code(
                    libarchive2_sys::archive_write_close(self.archive),
                    self.archive,
                )?;
                libarchive2_sys::archive_write_free(self.archive);
                self.archive = std::ptr::null_mut();
            }
        }
        Ok(())
    }
}

impl Drop for WriteDisk {
    fn drop(&mut self) {
        unsafe {
            if !self.archive.is_null() {
                libarchive2_sys::archive_write_close(self.archive);
                libarchive2_sys::archive_write_free(self.archive);
            }
        }
    }
}

// Note: Default implementation removed because disk writer creation can fail.
// Use WriteDisk::new() instead.
