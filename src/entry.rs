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
///
/// The lifetime parameter ensures the entry cannot outlive the archive
/// and that only one entry can be active at a time (enforced through
/// the mutable borrow of the archive when calling next_entry).
pub struct Entry<'a> {
    pub(crate) entry: *mut libarchive2_sys::archive_entry,
    pub(crate) _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Entry<'a> {
    /// Get the pathname of the entry
    ///
    /// Returns an owned String to ensure safety, as the underlying C string
    /// may be invalidated when the entry is modified or freed.
    ///
    /// This method first tries to get the UTF-8 pathname, but if that fails
    /// (which can happen with non-ASCII filenames in certain archive formats),
    /// it falls back to the raw pathname and uses lossy UTF-8 conversion.
    pub fn pathname(&self) -> Option<String> {
        unsafe {
            // First try the UTF-8 version
            let ptr = libarchive2_sys::archive_entry_pathname_utf8(self.entry);
            if !ptr.is_null()
                && let Ok(s) = CStr::from_ptr(ptr).to_str()
            {
                return Some(s.to_owned());
            }

            // Fall back to the raw pathname with lossy conversion
            let ptr = libarchive2_sys::archive_entry_pathname(self.entry);
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
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
    ///
    /// Returns an owned String to ensure safety, as the underlying C string
    /// may be invalidated when the entry is modified or freed.
    pub fn uname(&self) -> Option<String> {
        unsafe {
            // First try the UTF-8 version
            let ptr = libarchive2_sys::archive_entry_uname_utf8(self.entry);
            if !ptr.is_null()
                && let Ok(s) = CStr::from_ptr(ptr).to_str()
            {
                return Some(s.to_owned());
            }

            // Fall back to the raw uname with lossy conversion
            let ptr = libarchive2_sys::archive_entry_uname(self.entry);
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
            }
        }
    }

    /// Get the group name
    ///
    /// Returns an owned String to ensure safety, as the underlying C string
    /// may be invalidated when the entry is modified or freed.
    pub fn gname(&self) -> Option<String> {
        unsafe {
            // First try the UTF-8 version
            let ptr = libarchive2_sys::archive_entry_gname_utf8(self.entry);
            if !ptr.is_null()
                && let Ok(s) = CStr::from_ptr(ptr).to_str()
            {
                return Some(s.to_owned());
            }

            // Fall back to the raw gname with lossy conversion
            let ptr = libarchive2_sys::archive_entry_gname(self.entry);
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
            }
        }
    }

    /// Get the symlink target (for symbolic links)
    ///
    /// Returns an owned String to ensure safety, as the underlying C string
    /// may be invalidated when the entry is modified or freed.
    pub fn symlink(&self) -> Option<String> {
        unsafe {
            // First try the UTF-8 version
            let ptr = libarchive2_sys::archive_entry_symlink_utf8(self.entry);
            if !ptr.is_null()
                && let Ok(s) = CStr::from_ptr(ptr).to_str()
            {
                return Some(s.to_owned());
            }

            // Fall back to the raw symlink with lossy conversion
            let ptr = libarchive2_sys::archive_entry_symlink(self.entry);
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
            }
        }
    }

    /// Get the hardlink target
    ///
    /// Returns an owned String to ensure safety, as the underlying C string
    /// may be invalidated when the entry is modified or freed.
    pub fn hardlink(&self) -> Option<String> {
        unsafe {
            // First try the UTF-8 version
            let ptr = libarchive2_sys::archive_entry_hardlink_utf8(self.entry);
            if !ptr.is_null()
                && let Ok(s) = CStr::from_ptr(ptr).to_str()
            {
                return Some(s.to_owned());
            }

            // Fall back to the raw hardlink with lossy conversion
            let ptr = libarchive2_sys::archive_entry_hardlink(self.entry);
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
            }
        }
    }

    /// Get the access time
    pub fn atime(&self) -> Option<SystemTime> {
        unsafe {
            let sec = libarchive2_sys::archive_entry_atime(self.entry);
            let nsec = libarchive2_sys::archive_entry_atime_nsec(self.entry);
            if sec >= 0 {
                Some(SystemTime::UNIX_EPOCH + std::time::Duration::new(sec as u64, nsec as u32))
            } else {
                None
            }
        }
    }

    /// Get the creation time (birth time)
    ///
    /// Note: Not all archive formats and filesystems support birth time.
    pub fn birthtime(&self) -> Option<SystemTime> {
        unsafe {
            let sec = libarchive2_sys::archive_entry_birthtime(self.entry);
            let nsec = libarchive2_sys::archive_entry_birthtime_nsec(self.entry);
            if sec >= 0 {
                Some(SystemTime::UNIX_EPOCH + std::time::Duration::new(sec as u64, nsec as u32))
            } else {
                None
            }
        }
    }

    /// Get the status change time
    pub fn ctime(&self) -> Option<SystemTime> {
        unsafe {
            let sec = libarchive2_sys::archive_entry_ctime(self.entry);
            let nsec = libarchive2_sys::archive_entry_ctime_nsec(self.entry);
            if sec >= 0 {
                Some(SystemTime::UNIX_EPOCH + std::time::Duration::new(sec as u64, nsec as u32))
            } else {
                None
            }
        }
    }

    /// Get the device number (for block and character devices)
    pub fn dev(&self) -> Option<u64> {
        unsafe {
            if libarchive2_sys::archive_entry_dev_is_set(self.entry) != 0 {
                Some(libarchive2_sys::archive_entry_dev(self.entry) as u64)
            } else {
                None
            }
        }
    }

    /// Get the device major number
    ///
    /// Returns the device major number. For entries without a device,
    /// this may return 0 or an undefined value.
    pub fn devmajor(&self) -> u64 {
        unsafe { libarchive2_sys::archive_entry_devmajor(self.entry) as u64 }
    }

    /// Get the device minor number
    ///
    /// Returns the device minor number. For entries without a device,
    /// this may return 0 or an undefined value.
    pub fn devminor(&self) -> u64 {
        unsafe { libarchive2_sys::archive_entry_devminor(self.entry) as u64 }
    }

    /// Get the inode number
    ///
    /// Returns the inode number. If not set, returns 0.
    pub fn ino(&self) -> u64 {
        unsafe { libarchive2_sys::archive_entry_ino64(self.entry) as u64 }
    }

    /// Get the number of hard links
    ///
    /// Returns the number of hard links. If not set, returns 0.
    pub fn nlink(&self) -> u32 {
        unsafe { libarchive2_sys::archive_entry_nlink(self.entry) as u32 }
    }

    /// Get the device ID (for the filesystem containing the file)
    ///
    /// Returns the device ID. For entries without a device,
    /// this may return 0 or an undefined value.
    pub fn rdev(&self) -> u64 {
        unsafe { libarchive2_sys::archive_entry_rdev(self.entry) as u64 }
    }

    /// Get the major device number (for the filesystem containing the file)
    ///
    /// Returns the major device number. For entries without a device,
    /// this may return 0 or an undefined value.
    pub fn rdevmajor(&self) -> u64 {
        unsafe { libarchive2_sys::archive_entry_rdevmajor(self.entry) as u64 }
    }

    /// Get the minor device number (for the filesystem containing the file)
    ///
    /// Returns the minor device number. For entries without a device,
    /// this may return 0 or an undefined value.
    pub fn rdevminor(&self) -> u64 {
        unsafe { libarchive2_sys::archive_entry_rdevminor(self.entry) as u64 }
    }

    /// Get file flags (BSD-style file flags)
    ///
    /// Returns a tuple of (set flags, clear flags)
    pub fn fflags(&self) -> Option<(u64, u64)> {
        unsafe {
            let mut set: std::os::raw::c_ulong = 0;
            let mut clear: std::os::raw::c_ulong = 0;
            libarchive2_sys::archive_entry_fflags(self.entry, &mut set, &mut clear);
            Some((set as u64, clear as u64))
        }
    }

    /// Get file flags as a text string
    ///
    /// Returns an owned String to ensure safety, as the underlying C string
    /// may be invalidated when the entry is modified or freed.
    pub fn fflags_text(&self) -> Option<String> {
        unsafe {
            let ptr = libarchive2_sys::archive_entry_fflags_text(self.entry);
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_owned())
            }
        }
    }

    /// Check if entry is encrypted
    pub fn is_encrypted(&self) -> bool {
        unsafe { libarchive2_sys::archive_entry_is_encrypted(self.entry) != 0 }
    }

    /// Check if entry data is encrypted
    pub fn is_data_encrypted(&self) -> bool {
        unsafe { libarchive2_sys::archive_entry_is_data_encrypted(self.entry) != 0 }
    }

    /// Check if entry metadata is encrypted
    pub fn is_metadata_encrypted(&self) -> bool {
        unsafe { libarchive2_sys::archive_entry_is_metadata_encrypted(self.entry) != 0 }
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
        // SAFETY: entry is a valid pointer and file_type.to_mode() returns a valid mode value
        unsafe {
            libarchive2_sys::archive_entry_set_filetype(self.entry, file_type.to_mode());
        }
    }

    /// Set the file size
    pub fn set_size(&mut self, size: i64) {
        // SAFETY: entry is a valid pointer
        unsafe {
            libarchive2_sys::archive_entry_set_size(self.entry, size);
        }
    }

    /// Set the file permissions
    ///
    /// On platforms where permissions are stored as u16 (macOS, Windows, BSD),
    /// this will return an error if perm > 0xFFFF. Standard Unix permissions
    /// (0o777) are always safe on all platforms.
    ///
    /// # Platform Notes
    ///
    /// The underlying `mode_t` type varies by platform:
    /// - Linux (64-bit): u32
    /// - macOS/BSD/Windows: u16
    /// - Android: varies by architecture (u32 on 64-bit, u16 on 32-bit)
    pub fn set_perm(&mut self, perm: u32) -> Result<()> {
        unsafe {
            // Note: This platform detection is conservative. It assumes u32 on 64-bit Linux/Android
            // and u16 elsewhere. The libarchive2-sys crate should ideally provide type information
            // from its build script to make this more accurate.

            // Platforms with 32-bit mode_t: Linux and Android on 64-bit architectures
            #[cfg(all(
                any(target_os = "linux", target_os = "android"),
                any(
                    target_arch = "x86_64",
                    target_arch = "aarch64",
                    target_arch = "loongarch64",
                    target_arch = "riscv64"
                )
            ))]
            {
                libarchive2_sys::archive_entry_set_perm(self.entry, perm);
            }

            // All other platforms (macOS, BSD, Windows, 32-bit architectures) use 16-bit mode_t
            #[cfg(not(all(
                any(target_os = "linux", target_os = "android"),
                any(
                    target_arch = "x86_64",
                    target_arch = "aarch64",
                    target_arch = "loongarch64",
                    target_arch = "riscv64"
                )
            )))]
            {
                // SAFETY: We check that the value fits in u16 to prevent silent truncation
                // Standard Unix permissions (0o777 = 0x1FF) always fit in u16
                if perm > 0xFFFF {
                    return Err(Error::InvalidArgument(format!(
                        "Permission value 0x{:X} exceeds platform maximum 0xFFFF",
                        perm
                    )));
                }
                libarchive2_sys::archive_entry_set_perm(self.entry, perm as u16);
            }
        }
        Ok(())
    }

    /// Set the modification time
    ///
    /// # Platform Notes
    ///
    /// The underlying time_t type varies by platform:
    /// - Most Unix/Linux: i64 seconds, i64 nanoseconds
    /// - Windows: i64 seconds, i32 nanoseconds
    /// - Android 32-bit: i32 seconds, i32 nanoseconds
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

    /// Set the hardlink target
    pub fn set_hardlink(&mut self, target: &str) -> Result<()> {
        let c_target = CString::new(target).map_err(|_| {
            Error::InvalidArgument("Hardlink target contains null byte".to_string())
        })?;
        unsafe {
            libarchive2_sys::archive_entry_set_hardlink_utf8(self.entry, c_target.as_ptr());
        }
        Ok(())
    }

    /// Set the access time
    pub fn set_atime(&mut self, time: SystemTime) {
        if let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) {
            let nsec = duration.subsec_nanos();
            unsafe {
                #[cfg(all(target_os = "android", any(target_arch = "arm", target_arch = "x86")))]
                {
                    libarchive2_sys::archive_entry_set_atime(
                        self.entry,
                        duration.as_secs() as i32,
                        nsec as i32,
                    );
                }
                #[cfg(target_os = "windows")]
                {
                    libarchive2_sys::archive_entry_set_atime(
                        self.entry,
                        duration.as_secs() as i64,
                        nsec as i32,
                    );
                }
                #[cfg(not(any(
                    target_os = "windows",
                    all(target_os = "android", any(target_arch = "arm", target_arch = "x86"))
                )))]
                {
                    libarchive2_sys::archive_entry_set_atime(
                        self.entry,
                        duration.as_secs() as i64,
                        nsec as i64,
                    );
                }
            }
        }
    }

    /// Set the creation time (birth time)
    pub fn set_birthtime(&mut self, time: SystemTime) {
        if let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) {
            let nsec = duration.subsec_nanos();
            unsafe {
                #[cfg(all(target_os = "android", any(target_arch = "arm", target_arch = "x86")))]
                {
                    libarchive2_sys::archive_entry_set_birthtime(
                        self.entry,
                        duration.as_secs() as i32,
                        nsec as i32,
                    );
                }
                #[cfg(target_os = "windows")]
                {
                    libarchive2_sys::archive_entry_set_birthtime(
                        self.entry,
                        duration.as_secs() as i64,
                        nsec as i32,
                    );
                }
                #[cfg(not(any(
                    target_os = "windows",
                    all(target_os = "android", any(target_arch = "arm", target_arch = "x86"))
                )))]
                {
                    libarchive2_sys::archive_entry_set_birthtime(
                        self.entry,
                        duration.as_secs() as i64,
                        nsec as i64,
                    );
                }
            }
        }
    }

    /// Set the status change time
    pub fn set_ctime(&mut self, time: SystemTime) {
        if let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) {
            let nsec = duration.subsec_nanos();
            unsafe {
                #[cfg(all(target_os = "android", any(target_arch = "arm", target_arch = "x86")))]
                {
                    libarchive2_sys::archive_entry_set_ctime(
                        self.entry,
                        duration.as_secs() as i32,
                        nsec as i32,
                    );
                }
                #[cfg(target_os = "windows")]
                {
                    libarchive2_sys::archive_entry_set_ctime(
                        self.entry,
                        duration.as_secs() as i64,
                        nsec as i32,
                    );
                }
                #[cfg(not(any(
                    target_os = "windows",
                    all(target_os = "android", any(target_arch = "arm", target_arch = "x86"))
                )))]
                {
                    libarchive2_sys::archive_entry_set_ctime(
                        self.entry,
                        duration.as_secs() as i64,
                        nsec as i64,
                    );
                }
            }
        }
    }

    /// Set the device number
    pub fn set_dev(&mut self, dev: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_dev(self.entry, dev as _);
        }
    }

    /// Set the device major number
    pub fn set_devmajor(&mut self, major: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_devmajor(self.entry, major as _);
        }
    }

    /// Set the device minor number
    pub fn set_devminor(&mut self, minor: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_devminor(self.entry, minor as _);
        }
    }

    /// Set the inode number
    pub fn set_ino(&mut self, ino: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_ino64(self.entry, ino as i64);
        }
    }

    /// Set the number of hard links
    pub fn set_nlink(&mut self, nlink: u32) {
        unsafe {
            libarchive2_sys::archive_entry_set_nlink(self.entry, nlink);
        }
    }

    /// Set the device ID
    pub fn set_rdev(&mut self, rdev: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_rdev(self.entry, rdev as _);
        }
    }

    /// Set the major device number
    pub fn set_rdevmajor(&mut self, major: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_rdevmajor(self.entry, major as _);
        }
    }

    /// Set the minor device number
    pub fn set_rdevminor(&mut self, minor: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_rdevminor(self.entry, minor as _);
        }
    }

    /// Set file flags (BSD-style)
    pub fn set_fflags(&mut self, set: u64, clear: u64) {
        unsafe {
            libarchive2_sys::archive_entry_set_fflags(
                self.entry,
                set as std::os::raw::c_ulong,
                clear as std::os::raw::c_ulong,
            );
        }
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
