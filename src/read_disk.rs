//! Read files from disk into archive entries

use crate::entry::EntryMut;
use crate::error::{Error, Result};
use std::ffi::CString;
use std::path::Path;

/// Behavior flags for reading from disk
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadDiskFlags(i32);

impl ReadDiskFlags {
    /// No special behavior
    pub const NONE: ReadDiskFlags = ReadDiskFlags(0);

    /// Restore access time after reading
    pub const RESTORE_ATIME: ReadDiskFlags = ReadDiskFlags(0x0001);

    /// Honor nodump flag (skip files marked with nodump)
    pub const HONOR_NODUMP: ReadDiskFlags = ReadDiskFlags(0x0002);

    /// Use Mac copyfile for resource forks
    pub const MAC_COPYFILE: ReadDiskFlags = ReadDiskFlags(0x0004);

    /// Don't traverse mount points
    pub const NO_TRAVERSE_MOUNTS: ReadDiskFlags = ReadDiskFlags(0x0008);

    /// Don't read extended attributes
    pub const NO_XATTR: ReadDiskFlags = ReadDiskFlags(0x0010);

    /// Don't read ACLs
    pub const NO_ACL: ReadDiskFlags = ReadDiskFlags(0x0020);

    /// Don't read file flags
    pub const NO_FFLAGS: ReadDiskFlags = ReadDiskFlags(0x0040);

    /// Don't read sparse file information
    pub const NO_SPARSE: ReadDiskFlags = ReadDiskFlags(0x0080);

    /// Get the raw integer value
    pub fn bits(&self) -> i32 {
        self.0
    }
}

impl std::ops::BitOr for ReadDiskFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        ReadDiskFlags(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for ReadDiskFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Symlink handling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkMode {
    /// Follow all symlinks (like -L)
    Logical,
    /// Follow no symlinks (like -P)
    Physical,
    /// Follow symlinks on command line only (like -H)
    Hybrid,
}

/// Archive reader for reading files from disk
///
/// This provides the `archive_read_disk` API for reading file metadata
/// and content from the filesystem.
///
/// # Thread Safety
///
/// `ReadDisk` is `Send` but not `Sync`. You can transfer ownership between threads,
/// but cannot share references across threads.
pub struct ReadDisk {
    archive: *mut libarchive2_sys::archive,
}

// SAFETY: ReadDisk can be sent between threads because the archive pointer
// is owned exclusively by this instance and libarchive read_disk objects
// can be used from different threads (just not concurrently).
unsafe impl Send for ReadDisk {}

// Note: ReadDisk is NOT Sync because libarchive archives are not thread-safe
// for concurrent access.

impl ReadDisk {
    /// Create a new disk reader
    pub fn new() -> Result<Self> {
        unsafe {
            let archive = libarchive2_sys::archive_read_disk_new();
            if archive.is_null() {
                return Err(Error::NullPointer);
            }
            Ok(ReadDisk { archive })
        }
    }

    /// Set symlink handling mode
    pub fn set_symlink_mode(&mut self, mode: SymlinkMode) -> Result<()> {
        unsafe {
            let ret = match mode {
                SymlinkMode::Logical => {
                    libarchive2_sys::archive_read_disk_set_symlink_logical(self.archive)
                }
                SymlinkMode::Physical => {
                    libarchive2_sys::archive_read_disk_set_symlink_physical(self.archive)
                }
                SymlinkMode::Hybrid => {
                    libarchive2_sys::archive_read_disk_set_symlink_hybrid(self.archive)
                }
            };
            Error::from_return_code(ret, self.archive)?;
        }
        Ok(())
    }

    /// Set behavior flags
    pub fn set_behavior(&mut self, flags: ReadDiskFlags) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_disk_set_behavior(self.archive, flags.bits()),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Use standard user/group lookup functions
    pub fn set_standard_lookup(&mut self) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_disk_set_standard_lookup(self.archive),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Open a path for reading
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::InvalidArgument("Path contains invalid UTF-8".to_string()))?;
        let c_path = CString::new(path_str)
            .map_err(|_| Error::InvalidArgument("Path contains null byte".to_string()))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_disk_open(self.archive, c_path.as_ptr()),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Read the next entry from disk
    ///
    /// Returns `None` when there are no more entries.
    ///
    /// Note: The returned EntryMut is managed by libarchive and should not be freed
    /// manually. It becomes invalid when next_entry() is called again or when the
    /// ReadDisk is dropped.
    ///
    /// # Safety Considerations
    ///
    /// The entry's lifetime is tied to the ReadDisk instance. Do not use the entry
    /// after calling next_entry() again, as libarchive may reuse or free the memory.
    pub fn next_entry(&mut self) -> Result<Option<EntryMut>> {
        unsafe {
            // Create a new entry that will be populated by libarchive
            let entry_ptr = libarchive2_sys::archive_entry_new();
            if entry_ptr.is_null() {
                return Err(Error::NullPointer);
            }

            let ret = libarchive2_sys::archive_read_next_header2(self.archive, entry_ptr);

            if ret == libarchive2_sys::ARCHIVE_EOF as i32 {
                // Clean up the entry we created
                libarchive2_sys::archive_entry_free(entry_ptr);
                return Ok(None);
            }

            if let Err(e) = Error::from_return_code(ret, self.archive) {
                // Clean up the entry on error
                libarchive2_sys::archive_entry_free(entry_ptr);
                return Err(e);
            }

            // SAFETY: We created this entry and it's now populated by libarchive.
            // We own it and must free it when done.
            Ok(Some(EntryMut {
                entry: entry_ptr,
                owned: true,
            }))
        }
    }

    /// Request that current directory be descended into
    pub fn descend(&mut self) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_disk_descend(self.archive),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Check if current entry can be descended into
    pub fn can_descend(&self) -> bool {
        unsafe { libarchive2_sys::archive_read_disk_can_descend(self.archive) != 0 }
    }

    /// Close the disk reader
    pub fn close(mut self) -> Result<()> {
        unsafe {
            if !self.archive.is_null() {
                Error::from_return_code(
                    libarchive2_sys::archive_read_close(self.archive),
                    self.archive,
                )?;
                libarchive2_sys::archive_read_free(self.archive);
                self.archive = std::ptr::null_mut();
            }
        }
        Ok(())
    }
}

impl Drop for ReadDisk {
    fn drop(&mut self) {
        unsafe {
            if !self.archive.is_null() {
                libarchive2_sys::archive_read_close(self.archive);
                libarchive2_sys::archive_read_free(self.archive);
            }
        }
    }
}

// Note: Default implementation removed because disk reader creation can fail.
// Use ReadDisk::new() instead.
