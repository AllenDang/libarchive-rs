//! Archive entry types and operations

use crate::error::{Error, Result};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::time::SystemTime;

/// File type of an archive entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Regular file
    RegularFile,
    /// Directory
    Directory,
    /// Symbolic link
    SymbolicLink,
    /// Block device
    BlockDevice,
    /// Character device
    CharacterDevice,
    /// FIFO/named pipe
    Fifo,
    /// Socket
    Socket,
    /// Unknown type
    Unknown,
}

impl FileType {
    fn from_mode(mode: u32) -> Self {
        const S_IFMT: u32 = 0o170000;
        const S_IFREG: u32 = 0o100000;
        const S_IFDIR: u32 = 0o040000;
        const S_IFLNK: u32 = 0o120000;
        const S_IFBLK: u32 = 0o060000;
        const S_IFCHR: u32 = 0o020000;
        const S_IFIFO: u32 = 0o010000;
        const S_IFSOCK: u32 = 0o140000;

        match mode & S_IFMT {
            S_IFREG => FileType::RegularFile,
            S_IFDIR => FileType::Directory,
            S_IFLNK => FileType::SymbolicLink,
            S_IFBLK => FileType::BlockDevice,
            S_IFCHR => FileType::CharacterDevice,
            S_IFIFO => FileType::Fifo,
            S_IFSOCK => FileType::Socket,
            _ => FileType::Unknown,
        }
    }

    fn to_mode(self) -> u32 {
        const S_IFREG: u32 = 0o100000;
        const S_IFDIR: u32 = 0o040000;
        const S_IFLNK: u32 = 0o120000;
        const S_IFBLK: u32 = 0o060000;
        const S_IFCHR: u32 = 0o020000;
        const S_IFIFO: u32 = 0o010000;
        const S_IFSOCK: u32 = 0o140000;

        match self {
            FileType::RegularFile => S_IFREG,
            FileType::Directory => S_IFDIR,
            FileType::SymbolicLink => S_IFLNK,
            FileType::BlockDevice => S_IFBLK,
            FileType::CharacterDevice => S_IFCHR,
            FileType::Fifo => S_IFIFO,
            FileType::Socket => S_IFSOCK,
            FileType::Unknown => 0,
        }
    }
}

/// Immutable reference to an archive entry
pub struct Entry<'a> {
    pub(crate) entry: *mut libarchive2_sys::archive_entry,
    pub(crate) _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Entry<'a> {
    /// Get the pathname of the entry
    pub fn pathname(&self) -> Option<&str> {
        unsafe {
            let ptr = libarchive2_sys::archive_entry_pathname_utf8(self.entry);
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }

    /// Get the file type
    pub fn file_type(&self) -> FileType {
        unsafe {
            let mode = libarchive2_sys::archive_entry_filetype(self.entry);
            FileType::from_mode(mode as u32)
        }
    }

    /// Get the file size in bytes
    pub fn size(&self) -> i64 {
        unsafe { libarchive2_sys::archive_entry_size(self.entry) }
    }

    /// Get the file permissions (mode)
    pub fn mode(&self) -> u32 {
        unsafe { libarchive2_sys::archive_entry_perm(self.entry) as u32 }
    }

    /// Get the modification time
    pub fn mtime(&self) -> Option<SystemTime> {
        unsafe {
            let sec = libarchive2_sys::archive_entry_mtime(self.entry);
            let nsec = libarchive2_sys::archive_entry_mtime_nsec(self.entry);
            if sec >= 0 {
                Some(SystemTime::UNIX_EPOCH + std::time::Duration::new(sec as u64, nsec as u32))
            } else {
                None
            }
        }
    }

    /// Get the user ID
    pub fn uid(&self) -> Option<u64> {
        unsafe {
            if libarchive2_sys::archive_entry_uid_is_set(self.entry) != 0 {
                Some(libarchive2_sys::archive_entry_uid(self.entry) as u64)
            } else {
                None
            }
        }
    }

    /// Get the group ID
    pub fn gid(&self) -> Option<u64> {
        unsafe {
            if libarchive2_sys::archive_entry_gid_is_set(self.entry) != 0 {
                Some(libarchive2_sys::archive_entry_gid(self.entry) as u64)
            } else {
                None
            }
        }
    }

    /// Get the user name
    pub fn uname(&self) -> Option<&str> {
        unsafe {
            let ptr = libarchive2_sys::archive_entry_uname_utf8(self.entry);
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }

    /// Get the group name
    pub fn gname(&self) -> Option<&str> {
        unsafe {
            let ptr = libarchive2_sys::archive_entry_gname_utf8(self.entry);
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }

    /// Get the symlink target (for symbolic links)
    pub fn symlink(&self) -> Option<&str> {
        unsafe {
            let ptr = libarchive2_sys::archive_entry_symlink_utf8(self.entry);
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }

    /// Get the hardlink target
    pub fn hardlink(&self) -> Option<&str> {
        unsafe {
            let ptr = libarchive2_sys::archive_entry_hardlink_utf8(self.entry);
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }
}

/// Mutable reference to an archive entry for building/writing
pub struct EntryMut {
    pub(crate) entry: *mut libarchive2_sys::archive_entry,
    pub(crate) owned: bool,
}

impl EntryMut {
    /// Create a new entry
    pub fn new() -> Self {
        unsafe {
            let entry = libarchive2_sys::archive_entry_new();
            EntryMut { entry, owned: true }
        }
    }

    /// Set the pathname
    pub fn set_pathname<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::InvalidArgument("Path contains invalid UTF-8".to_string()))?;
        let c_path = CString::new(path_str)
            .map_err(|_| Error::InvalidArgument("Path contains null byte".to_string()))?;

        unsafe {
            libarchive2_sys::archive_entry_set_pathname_utf8(self.entry, c_path.as_ptr());
        }
        Ok(())
    }

    /// Set the file type
    pub fn set_file_type(&mut self, file_type: FileType) {
        unsafe {
            libarchive2_sys::archive_entry_set_filetype(self.entry, file_type.to_mode());
        }
    }

    /// Set the file size
    pub fn set_size(&mut self, size: i64) {
        unsafe {
            libarchive2_sys::archive_entry_set_size(self.entry, size);
        }
    }

    /// Set the file permissions
    pub fn set_perm(&mut self, perm: u32) {
        unsafe {
            // Linux (x86_64, aarch64) and Android (x86_64, aarch64) use u32
            // macOS/Windows/iOS and Android (armv7, x86) use u16
            #[cfg(all(
                any(target_os = "linux", target_os = "android"),
                any(target_arch = "x86_64", target_arch = "aarch64")
            ))]
            {
                libarchive2_sys::archive_entry_set_perm(self.entry, perm);
            }
            #[cfg(not(all(
                any(target_os = "linux", target_os = "android"),
                any(target_arch = "x86_64", target_arch = "aarch64")
            )))]
            {
                libarchive2_sys::archive_entry_set_perm(self.entry, perm as u16);
            }
        }
    }

    /// Set the modification time
    pub fn set_mtime(&mut self, time: SystemTime) {
        if let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) {
            let nsec = duration.subsec_nanos();
            unsafe {
                // Android 32-bit (armv7, x86) uses i32 for both sec and nsec
                #[cfg(all(target_os = "android", any(target_arch = "arm", target_arch = "x86")))]
                {
                    libarchive2_sys::archive_entry_set_mtime(
                        self.entry,
                        duration.as_secs() as i32,
                        nsec as i32,
                    );
                }
                // Windows uses i64 sec, i32 nsec
                #[cfg(target_os = "windows")]
                {
                    libarchive2_sys::archive_entry_set_mtime(
                        self.entry,
                        duration.as_secs() as i64,
                        nsec as i32,
                    );
                }
                // Unix platforms and Android 64-bit use i64 for both
                #[cfg(not(any(
                    target_os = "windows",
                    all(target_os = "android", any(target_arch = "arm", target_arch = "x86"))
                )))]
                {
                    libarchive2_sys::archive_entry_set_mtime(
                        self.entry,
                        duration.as_secs() as i64,
                        nsec as i64,
                    );
                }
            }
        }
    }

    /// Set the user ID
    pub fn set_uid(&mut self, uid: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_uid(self.entry, uid as i64);
        }
    }

    /// Set the group ID
    pub fn set_gid(&mut self, gid: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_gid(self.entry, gid as i64);
        }
    }

    /// Set the user name
    pub fn set_uname(&mut self, uname: &str) -> Result<()> {
        let c_uname = CString::new(uname)
            .map_err(|_| Error::InvalidArgument("Username contains null byte".to_string()))?;
        unsafe {
            libarchive2_sys::archive_entry_set_uname_utf8(self.entry, c_uname.as_ptr());
        }
        Ok(())
    }

    /// Set the group name
    pub fn set_gname(&mut self, gname: &str) -> Result<()> {
        let c_gname = CString::new(gname)
            .map_err(|_| Error::InvalidArgument("Group name contains null byte".to_string()))?;
        unsafe {
            libarchive2_sys::archive_entry_set_gname_utf8(self.entry, c_gname.as_ptr());
        }
        Ok(())
    }

    /// Set the symlink target
    pub fn set_symlink(&mut self, target: &str) -> Result<()> {
        let c_target = CString::new(target)
            .map_err(|_| Error::InvalidArgument("Symlink target contains null byte".to_string()))?;
        unsafe {
            libarchive2_sys::archive_entry_set_symlink_utf8(self.entry, c_target.as_ptr());
        }
        Ok(())
    }

    /// Get an immutable view of this entry
    pub fn as_entry(&self) -> Entry<'_> {
        Entry {
            entry: self.entry,
            _marker: std::marker::PhantomData,
        }
    }
}

impl Default for EntryMut {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EntryMut {
    fn drop(&mut self) {
        if self.owned && !self.entry.is_null() {
            unsafe {
                libarchive2_sys::archive_entry_free(self.entry);
            }
        }
    }
}
