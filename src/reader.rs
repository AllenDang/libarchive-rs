//! Archive reading functionality

use crate::entry::Entry;
use crate::error::{Error, Result};
use crate::format::{CompressionFormat, ReadFormat};
use std::ffi::CString;
use std::path::Path;
use std::ptr;

/// Archive reader with RAII resource management
///
/// The lifetime parameter 'a represents borrowed data (e.g., when reading from memory).
/// For file-based archives, use 'static lifetime.
///
/// # Thread Safety
///
/// `ReadArchive` is `Send` but not `Sync`. You can transfer ownership between threads,
/// but cannot share references across threads. This matches libarchive's thread safety
/// guarantees: archive objects should not be shared between threads, but can be moved.
pub struct ReadArchive<'a> {
    archive: *mut libarchive2_sys::archive,
    _callback_data: Option<(*mut std::ffi::c_void, crate::callbacks::DropFn)>,
    _phantom: std::marker::PhantomData<&'a [u8]>,
}

// SAFETY: ReadArchive can be sent between threads because:
// 1. The archive pointer is owned exclusively by this instance
// 2. libarchive archive objects can be used from different threads (just not concurrently)
// 3. The callback data is also owned exclusively
// 4. The phantom data only tracks lifetimes, not actual data
unsafe impl<'a> Send for ReadArchive<'a> {}

// Note: ReadArchive is NOT Sync because libarchive archives are not thread-safe
// for concurrent access. Multiple threads cannot safely call methods on the same
// archive at the same time.

impl<'a> ReadArchive<'a> {
    /// Create a new archive reader
    pub fn new() -> Result<Self> {
        unsafe {
            let archive = libarchive2_sys::archive_read_new();
            if archive.is_null() {
                return Err(Error::NullPointer);
            }
            Ok(ReadArchive {
                archive,
                _callback_data: None,
                _phantom: std::marker::PhantomData,
            })
        }
    }

    /// Get the raw archive pointer (for internal use by sibling modules)
    pub(crate) fn archive(&self) -> *mut libarchive2_sys::archive {
        self.archive
    }

    /// Open an archive file for reading
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut reader = Self::new()?;
        reader.support_filter_all()?;
        reader.support_format_all()?;

        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::InvalidArgument("Path contains invalid UTF-8".to_string()))?;
        let c_path = CString::new(path_str)
            .map_err(|_| Error::InvalidArgument("Path contains null byte".to_string()))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_open_filename(reader.archive, c_path.as_ptr(), 10240),
                reader.archive,
            )?;
        }

        Ok(reader)
    }

    /// Open a multi-volume archive from multiple files
    ///
    /// This method allows reading archives that are split across multiple files.
    /// The files will be read in the order provided.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ReadArchive;
    ///
    /// // Read a multi-volume RAR archive split into parts
    /// let parts = vec!["archive.part1.rar", "archive.part2.rar", "archive.part3.rar"];
    /// let mut archive = ReadArchive::open_filenames(&parts)?;
    ///
    /// while let Some(entry) = archive.next_entry()? {
    ///     println!("Entry: {}", entry.pathname().unwrap_or_default());
    ///     // Process entry...
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Notes
    ///
    /// - The files must be provided in the correct order
    /// - All files must remain accessible for the lifetime of the ReadArchive
    /// - This is commonly used for RAR archives split into multiple parts
    pub fn open_filenames<P: AsRef<Path>>(paths: &[P]) -> Result<Self> {
        // Validate that at least one path is provided
        if paths.is_empty() {
            return Err(Error::InvalidArgument(
                "At least one file path must be provided".to_string(),
            ));
        }

        let mut reader = Self::new()?;
        reader.support_filter_all()?;
        reader.support_format_all()?;

        // Convert paths to C strings and collect them
        let c_paths: Result<Vec<CString>> = paths
            .iter()
            .map(|p| {
                let path_str = p.as_ref().to_str().ok_or_else(|| {
                    Error::InvalidArgument("Path contains invalid UTF-8".to_string())
                })?;
                CString::new(path_str)
                    .map_err(|_| Error::InvalidArgument("Path contains null byte".to_string()))
            })
            .collect();
        let c_paths = c_paths?;

        // Create null-terminated array of pointers
        let mut c_path_ptrs: Vec<*const std::os::raw::c_char> =
            c_paths.iter().map(|s| s.as_ptr()).collect();
        c_path_ptrs.push(std::ptr::null()); // Null terminator

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_open_filenames(
                    reader.archive,
                    c_path_ptrs.as_mut_ptr(),
                    10240,
                ),
                reader.archive,
            )?;
        }

        Ok(reader)
    }

    /// Open an encrypted multi-volume archive from multiple files with a passphrase
    ///
    /// This method combines multi-volume archive support with password protection,
    /// allowing you to read encrypted archives that are split across multiple files.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ReadArchive;
    ///
    /// // Read an encrypted multi-volume RAR archive
    /// let parts = vec!["secret.part1.rar", "secret.part2.rar", "secret.part3.rar"];
    /// let mut archive = ReadArchive::open_filenames_with_passphrase(&parts, "my_password")?;
    ///
    /// while let Some(entry) = archive.next_entry()? {
    ///     println!("Entry: {}", entry.pathname().unwrap_or_default());
    ///     // Process entry...
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Notes
    ///
    /// - The files must be provided in the correct order
    /// - All files must remain accessible for the lifetime of the ReadArchive
    /// - This is commonly used for encrypted RAR archives split into multiple parts
    /// - You can call `add_passphrase()` multiple times before opening to try multiple passwords
    pub fn open_filenames_with_passphrase<P: AsRef<Path>>(
        paths: &[P],
        passphrase: &str,
    ) -> Result<Self> {
        // Validate that at least one path is provided
        if paths.is_empty() {
            return Err(Error::InvalidArgument(
                "At least one file path must be provided".to_string(),
            ));
        }

        let mut reader = Self::new()?;
        reader.support_filter_all()?;
        reader.support_format_all()?;

        // Add passphrase before opening the archive
        reader.add_passphrase(passphrase)?;

        // Convert paths to C strings and collect them
        let c_paths: Result<Vec<CString>> = paths
            .iter()
            .map(|p| {
                let path_str = p.as_ref().to_str().ok_or_else(|| {
                    Error::InvalidArgument("Path contains invalid UTF-8".to_string())
                })?;
                CString::new(path_str)
                    .map_err(|_| Error::InvalidArgument("Path contains null byte".to_string()))
            })
            .collect();
        let c_paths = c_paths?;

        // Create null-terminated array of pointers
        let mut c_path_ptrs: Vec<*const std::os::raw::c_char> =
            c_paths.iter().map(|s| s.as_ptr()).collect();
        c_path_ptrs.push(std::ptr::null()); // Null terminator

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_open_filenames(
                    reader.archive,
                    c_path_ptrs.as_mut_ptr(),
                    10240,
                ),
                reader.archive,
            )?;
        }

        Ok(reader)
    }

    /// Open an archive from memory
    ///
    /// The data must remain valid for the lifetime of the ReadArchive.
    /// The lifetime 'a ensures that the data is not dropped while the archive is reading.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ReadArchive;
    ///
    /// let data = std::fs::read("archive.tar.gz")?;
    /// let mut archive = ReadArchive::open_memory(&data)?;
    /// // archive borrows data, so data cannot be dropped until archive is dropped
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open_memory(data: &'a [u8]) -> Result<Self> {
        let mut reader = Self::new()?;
        reader.support_filter_all()?;
        reader.support_format_all()?;

        unsafe {
            // SAFETY: The data slice is valid for lifetime 'a, which is tied to
            // the ReadArchive lifetime via the _phantom field. This ensures the
            // data cannot be dropped while libarchive is using it.
            Error::from_return_code(
                libarchive2_sys::archive_read_open_memory(
                    reader.archive,
                    data.as_ptr() as *const std::os::raw::c_void,
                    data.len(),
                ),
                reader.archive,
            )?;
        }

        Ok(reader)
    }

    /// Open an archive from a file descriptor
    ///
    /// # Safety
    /// The file descriptor must be valid and remain open for the lifetime of the archive.
    /// The archive will not close the file descriptor when dropped.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ReadArchive;
    /// use std::fs::File;
    /// use std::os::unix::io::AsRawFd;
    ///
    /// let file = File::open("archive.tar.gz")?;
    /// let mut archive = ReadArchive::open_fd(file.as_raw_fd())?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[cfg(unix)]
    pub fn open_fd(fd: std::os::unix::io::RawFd) -> Result<Self> {
        let mut reader = Self::new()?;
        reader.support_filter_all()?;
        reader.support_format_all()?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_open_fd(reader.archive, fd, 10240),
                reader.archive,
            )?;
        }

        Ok(reader)
    }

    /// Open an archive from a file descriptor (Windows)
    ///
    /// # Safety
    /// The file descriptor must be valid and remain open for the lifetime of the archive.
    /// The archive will not close the file descriptor when dropped.
    #[cfg(windows)]
    pub fn open_fd(fd: std::os::windows::io::RawHandle) -> Result<Self> {
        let mut reader = Self::new()?;
        reader.support_filter_all()?;
        reader.support_format_all()?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_open_fd(
                    reader.archive,
                    fd as std::os::raw::c_int,
                    10240,
                ),
                reader.archive,
            )?;
        }

        Ok(reader)
    }

    /// Open an encrypted archive file with a passphrase
    ///
    /// This is a convenience method that combines `open()` with `add_passphrase()`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ReadArchive;
    ///
    /// let mut archive = ReadArchive::open_with_passphrase(
    ///     "encrypted.zip",
    ///     "my_password"
    /// ).unwrap();
    /// ```
    pub fn open_with_passphrase<P: AsRef<Path>>(path: P, passphrase: &str) -> Result<Self> {
        let mut reader = Self::new()?;
        reader.support_filter_all()?;
        reader.support_format_all()?;
        reader.add_passphrase(passphrase)?;

        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::InvalidArgument("Path contains invalid UTF-8".to_string()))?;
        let c_path = CString::new(path_str)
            .map_err(|_| Error::InvalidArgument("Path contains null byte".to_string()))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_open_filename(reader.archive, c_path.as_ptr(), 10240),
                reader.archive,
            )?;
        }

        Ok(reader)
    }

    /// Open an archive using a custom callback
    ///
    /// This allows reading archives from custom data sources like network streams,
    /// encrypted files, or any other source that can be wrapped in a Read trait.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::{ReadArchive, CallbackReader};
    /// use std::fs::File;
    ///
    /// let file = File::open("archive.tar.gz")?;
    /// let callback = CallbackReader::new(file);
    /// let mut archive = ReadArchive::open_callback(callback)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open_callback<R: std::io::Read + 'static>(
        callback: crate::callbacks::CallbackReader<R>,
    ) -> Result<Self> {
        let mut reader = Self::new()?;
        reader.support_filter_all()?;
        reader.support_format_all()?;

        let (client_data, read_cb, close_cb, drop_fn) = callback.into_raw_parts();

        unsafe {
            // SAFETY: The function pointers returned from into_raw_parts are guaranteed
            // to have the correct signatures for the libarchive callback functions.
            // We cast them from *const c_void back to their proper function pointer types.
            type ReadFn = unsafe extern "C" fn(
                *mut libarchive2_sys::archive,
                *mut std::ffi::c_void,
                *mut *const std::ffi::c_void,
            ) -> isize;
            type CloseFn = unsafe extern "C" fn(
                *mut libarchive2_sys::archive,
                *mut std::ffi::c_void,
            ) -> std::os::raw::c_int;

            // Cast from void pointer to function pointer
            let read_fn = Some(std::mem::transmute::<*const std::ffi::c_void, ReadFn>(
                read_cb,
            ));
            let close_fn = Some(std::mem::transmute::<*const std::ffi::c_void, CloseFn>(
                close_cb,
            ));

            Error::from_return_code(
                libarchive2_sys::archive_read_open(
                    reader.archive,
                    client_data,
                    None,
                    read_fn,
                    close_fn,
                ),
                reader.archive,
            )?;
        }

        reader._callback_data = Some((client_data, drop_fn));
        Ok(reader)
    }

    /// Enable support for all compression filters
    pub fn support_filter_all(&mut self) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_support_filter_all(self.archive),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Enable support for a specific compression filter
    pub fn support_filter(&mut self, filter: CompressionFormat) -> Result<()> {
        unsafe {
            let ret = match filter {
                CompressionFormat::None => {
                    libarchive2_sys::archive_read_support_filter_none(self.archive)
                }
                CompressionFormat::Gzip => {
                    libarchive2_sys::archive_read_support_filter_gzip(self.archive)
                }
                CompressionFormat::Bzip2 => {
                    libarchive2_sys::archive_read_support_filter_bzip2(self.archive)
                }
                CompressionFormat::Xz => {
                    libarchive2_sys::archive_read_support_filter_xz(self.archive)
                }
                CompressionFormat::Zstd => {
                    libarchive2_sys::archive_read_support_filter_zstd(self.archive)
                }
                CompressionFormat::Lz4 => {
                    libarchive2_sys::archive_read_support_filter_lz4(self.archive)
                }
                CompressionFormat::Compress => {
                    libarchive2_sys::archive_read_support_filter_compress(self.archive)
                }
                CompressionFormat::UuEncode => {
                    libarchive2_sys::archive_read_support_filter_uu(self.archive)
                }
                CompressionFormat::Lrzip => {
                    libarchive2_sys::archive_read_support_filter_lrzip(self.archive)
                }
                CompressionFormat::Lzop => {
                    libarchive2_sys::archive_read_support_filter_lzop(self.archive)
                }
                CompressionFormat::Grzip => {
                    libarchive2_sys::archive_read_support_filter_grzip(self.archive)
                }
                _ => {
                    return Err(Error::InvalidArgument(format!(
                        "Unsupported filter: {:?}",
                        filter
                    )));
                }
            };
            Error::from_return_code(ret, self.archive)?;
        }
        Ok(())
    }

    /// Enable support for all archive formats
    pub fn support_format_all(&mut self) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_support_format_all(self.archive),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Enable support for a specific archive format
    pub fn support_format(&mut self, format: ReadFormat) -> Result<()> {
        unsafe {
            let ret = match format {
                ReadFormat::All => libarchive2_sys::archive_read_support_format_all(self.archive),
                ReadFormat::Format(fmt) => {
                    use crate::format::ArchiveFormat;
                    match fmt {
                        ArchiveFormat::Tar
                        | ArchiveFormat::TarGnu
                        | ArchiveFormat::TarPax
                        | ArchiveFormat::TarPaxRestricted
                        | ArchiveFormat::TarUstar => {
                            libarchive2_sys::archive_read_support_format_tar(self.archive)
                        }
                        ArchiveFormat::Zip => {
                            libarchive2_sys::archive_read_support_format_zip(self.archive)
                        }
                        ArchiveFormat::SevenZip => {
                            libarchive2_sys::archive_read_support_format_7zip(self.archive)
                        }
                        ArchiveFormat::Ar => {
                            libarchive2_sys::archive_read_support_format_ar(self.archive)
                        }
                        ArchiveFormat::Cpio
                        | ArchiveFormat::CpioNewc
                        | ArchiveFormat::CpioOdc
                        | ArchiveFormat::CpioBin => {
                            libarchive2_sys::archive_read_support_format_cpio(self.archive)
                        }
                        ArchiveFormat::Iso9660 => {
                            libarchive2_sys::archive_read_support_format_iso9660(self.archive)
                        }
                        ArchiveFormat::Xar => {
                            libarchive2_sys::archive_read_support_format_xar(self.archive)
                        }
                        ArchiveFormat::Mtree => {
                            libarchive2_sys::archive_read_support_format_mtree(self.archive)
                        }
                        ArchiveFormat::Raw => {
                            libarchive2_sys::archive_read_support_format_raw(self.archive)
                        }
                        ArchiveFormat::Warc => {
                            libarchive2_sys::archive_read_support_format_warc(self.archive)
                        }
                        ArchiveFormat::Rar => {
                            libarchive2_sys::archive_read_support_format_rar(self.archive)
                        }
                        ArchiveFormat::Rar5 => {
                            libarchive2_sys::archive_read_support_format_rar5(self.archive)
                        }
                        ArchiveFormat::Lha => {
                            libarchive2_sys::archive_read_support_format_lha(self.archive)
                        }
                        ArchiveFormat::Cab => {
                            libarchive2_sys::archive_read_support_format_cab(self.archive)
                        }
                        _ => {
                            return Err(Error::InvalidArgument(format!(
                                "Unsupported format: {:?}",
                                fmt
                            )));
                        }
                    }
                }
            };
            Error::from_return_code(ret, self.archive)?;
        }
        Ok(())
    }

    /// Add a passphrase for decrypting encrypted archives
    ///
    /// This method can be called multiple times to add multiple passphrases.
    /// libarchive will try each passphrase in the order they were added.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ReadArchive;
    ///
    /// let mut archive = ReadArchive::new().unwrap();
    /// archive.add_passphrase("my_password").unwrap();
    /// ```
    pub fn add_passphrase(&mut self, passphrase: &str) -> Result<()> {
        let c_passphrase = CString::new(passphrase)
            .map_err(|_| Error::InvalidArgument("Passphrase contains null byte".to_string()))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_add_passphrase(self.archive, c_passphrase.as_ptr()),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Set a format-specific option
    ///
    /// This allows fine-grained control over format-specific features during reading.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ReadArchive;
    ///
    /// let mut archive = ReadArchive::new().unwrap();
    /// archive.set_option("zip", "compat-2x", "1").unwrap();
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_option(&mut self, module: &str, option: &str, value: &str) -> Result<()> {
        let c_module = CString::new(module)
            .map_err(|_| Error::InvalidArgument("Module name contains null byte".to_string()))?;
        let c_option = CString::new(option)
            .map_err(|_| Error::InvalidArgument("Option name contains null byte".to_string()))?;
        let c_value = CString::new(value)
            .map_err(|_| Error::InvalidArgument("Option value contains null byte".to_string()))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_set_option(
                    self.archive,
                    c_module.as_ptr(),
                    c_option.as_ptr(),
                    c_value.as_ptr(),
                ),
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Read the next entry header
    ///
    /// Returns `None` when there are no more entries
    pub fn next_entry(&mut self) -> Result<Option<Entry<'_>>> {
        // Set locale to UTF-8 to handle non-ASCII filenames correctly
        let _guard = crate::locale::UTF8LocaleGuard::new();

        unsafe {
            let mut entry: *mut libarchive2_sys::archive_entry = ptr::null_mut();
            let ret = libarchive2_sys::archive_read_next_header(self.archive, &mut entry);

            if ret == libarchive2_sys::ARCHIVE_EOF as i32 {
                return Ok(None);
            }

            Error::from_return_code(ret, self.archive)?;

            Ok(Some(Entry {
                entry,
                _marker: std::marker::PhantomData,
            }))
        }
    }

    /// Read data from the current entry
    pub fn read_data(&mut self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            let ret = libarchive2_sys::archive_read_data(
                self.archive,
                buf.as_mut_ptr() as *mut std::os::raw::c_void,
                buf.len(),
            );

            if ret < 0 {
                Err(Error::from_archive(self.archive))
            } else {
                Ok(ret as usize)
            }
        }
    }

    /// Read all data from the current entry into a vector
    pub fn read_data_to_vec(&mut self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut buf = vec![0u8; 8192];

        loop {
            let n = self.read_data(&mut buf)?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
        }

        Ok(data)
    }

    /// Skip the data for the current entry
    pub fn skip_data(&mut self) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_data_skip(self.archive) as i32,
                self.archive,
            )?;
        }
        Ok(())
    }

    /// Seek within the current entry's data
    ///
    /// This allows random access within an entry's data. Not all archive formats
    /// and compression methods support seeking.
    ///
    /// # Arguments
    ///
    /// * `offset` - The offset to seek to
    /// * `whence` - The seek mode (0 = SEEK_SET, 1 = SEEK_CUR, 2 = SEEK_END)
    ///
    /// Returns the new position within the entry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ReadArchive;
    ///
    /// let mut archive = ReadArchive::open("archive.tar")?;
    /// if let Some(entry) = archive.next_entry()? {
    ///     // Seek to position 100 from start
    ///     archive.seek(100, 0)?;
    ///
    ///     // Seek forward 50 bytes
    ///     archive.seek(50, 1)?;
    ///
    ///     // Seek to 10 bytes before end
    ///     archive.seek(-10, 2)?;
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn seek(&mut self, offset: i64, whence: i32) -> Result<i64> {
        unsafe {
            let pos = libarchive2_sys::archive_seek_data(self.archive, offset, whence);
            if pos < 0 {
                Err(Error::from_archive(self.archive))
            } else {
                Ok(pos)
            }
        }
    }

    /// Check if the current entry supports data block operations
    ///
    /// Returns true if you can use read_data_block on this entry.
    pub fn has_data_block(&self) -> bool {
        unsafe { libarchive2_sys::archive_read_has_encrypted_entries(self.archive) > 0 }
    }

    /// Read the next data block from the current entry
    ///
    /// This is useful for sparse files or when you need fine-grained control over
    /// reading. Returns the offset within the entry and the data block.
    ///
    /// # Returns
    ///
    /// - `Ok(Some((offset, data)))` - Next data block with its offset
    /// - `Ok(None)` - End of entry data
    /// - `Err(...)` - Error occurred
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ReadArchive;
    ///
    /// let mut archive = ReadArchive::open("sparse.tar")?;
    /// if let Some(entry) = archive.next_entry()? {
    ///     // Read blocks with their offsets (useful for sparse files)
    ///     while let Some((offset, data)) = archive.read_data_block()? {
    ///         println!("Block at offset {}: {} bytes", offset, data.len());
    ///         // Write to disk at specific offset if needed
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn read_data_block(&mut self) -> Result<Option<(i64, Vec<u8>)>> {
        unsafe {
            let mut buffer: *const std::os::raw::c_void = std::ptr::null();
            let mut size: usize = 0;
            let mut offset: i64 = 0;

            let ret = libarchive2_sys::archive_read_data_block(
                self.archive,
                &mut buffer,
                &mut size,
                &mut offset,
            );

            if ret == libarchive2_sys::ARCHIVE_EOF as i32 {
                return Ok(None);
            }

            if ret == libarchive2_sys::ARCHIVE_OK as i32 {
                if size == 0 {
                    return Ok(None);
                }

                // SAFETY: libarchive guarantees buffer is valid and contains 'size' bytes
                // We copy the data to owned Vec to ensure memory safety
                let data = std::slice::from_raw_parts(buffer as *const u8, size).to_vec();
                Ok(Some((offset, data)))
            } else {
                Err(Error::from_archive(self.archive))
            }
        }
    }

    /// Extract the current entry to disk
    ///
    /// This is a convenience method that extracts entries with commonly used flags.
    /// For more control, use `extract_with_flags()`.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// use libarchive2::ReadArchive;
    ///
    /// let mut archive = ReadArchive::open("archive.tar.gz").unwrap();
    /// while let Some(entry) = archive.next_entry().unwrap() {
    ///     // Note: This will not compile due to borrow checker restrictions.
    ///     // See extract_with_flags for a working example using manual loop.
    ///     archive.extract(&entry, ".").unwrap();
    /// }
    /// ```
    pub fn extract<P: AsRef<Path>>(&mut self, entry: &Entry, dest: P) -> Result<()> {
        use crate::extract::ExtractFlags;
        let flags = ExtractFlags::TIME
            | ExtractFlags::PERM
            | ExtractFlags::ACL
            | ExtractFlags::FFLAGS
            | ExtractFlags::XATTR;
        self.extract_with_flags(entry, dest, flags)
    }

    /// Extract the current entry to disk with specific flags
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// use libarchive2::{ReadArchive, ExtractFlags};
    ///
    /// let mut archive = ReadArchive::open("archive.tar.gz").unwrap();
    /// let flags = ExtractFlags::TIME | ExtractFlags::PERM | ExtractFlags::SECURE_SYMLINKS;
    ///
    /// while let Some(entry) = archive.next_entry().unwrap() {
    ///     // Note: This will not compile due to borrow checker restrictions.
    ///     // The entry borrows the archive mutably, and extract also needs a mutable borrow.
    ///     // This is a known API limitation.
    ///     archive.extract_with_flags(&entry, ".", flags).unwrap();
    /// }
    /// ```
    pub fn extract_with_flags<P: AsRef<Path>>(
        &mut self,
        entry: &Entry,
        dest: P,
        flags: crate::extract::ExtractFlags,
    ) -> Result<()> {
        use crate::extract::WriteDisk;
        let mut disk = WriteDisk::new()?;
        disk.set_options(flags)?;
        disk.set_standard_lookup()?;

        // Update entry pathname to be relative to destination
        let dest_path = dest.as_ref();
        let entry_path = entry.pathname().unwrap_or_default();
        let full_path = dest_path.join(entry_path);

        // Create a new entry with updated path
        let mut new_entry = crate::entry::EntryMut::new();
        new_entry.set_pathname(&full_path)?;

        // Copy entry metadata
        new_entry.set_file_type(entry.file_type());
        new_entry.set_size(entry.size());
        new_entry.set_perm(entry.mode())?;
        if let Some(mtime) = entry.mtime() {
            new_entry.set_mtime(mtime);
        }
        if let Some(uid) = entry.uid() {
            new_entry.set_uid(uid);
        }
        if let Some(gid) = entry.gid() {
            new_entry.set_gid(gid);
        }
        if let Some(uname) = entry.uname() {
            new_entry.set_uname(&uname)?;
        }
        if let Some(gname) = entry.gname() {
            new_entry.set_gname(&gname)?;
        }
        if let Some(symlink) = entry.symlink() {
            new_entry.set_symlink(&symlink)?;
        }

        disk.write_header(&new_entry)?;

        // Copy data
        let mut buf = vec![0u8; 8192];
        loop {
            let n = self.read_data(&mut buf)?;
            if n == 0 {
                break;
            }
            disk.write_data(&buf[..n])?;
        }

        disk.finish_entry()?;
        Ok(())
    }
}

/// `std::io::Read` implementation for reading data from the current archive entry.
///
/// This allows using `ReadArchive` with anything that accepts a `Read` trait object,
/// such as `io::copy()` or `io::read_to_string()`. You must call
/// [`next_entry()`](ReadArchive::next_entry) before reading data.
///
/// # Examples
///
/// ```no_run
/// use std::io::Read;
/// use libarchive2::ReadArchive;
///
/// let mut archive = ReadArchive::open("archive.tar.gz")?;
/// while let Some(entry) = archive.next_entry()? {
///     let name = entry.pathname().unwrap_or_default();
///     let mut contents = String::new();
///     archive.read_to_string(&mut contents)?;
///     println!("{}: {}", name, contents.len());
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
impl<'a> std::io::Read for ReadArchive<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read_data(buf).map_err(|e| {
            std::io::Error::other(e.to_string())
        })
    }
}

impl<'a> Drop for ReadArchive<'a> {
    fn drop(&mut self) {
        unsafe {
            if !self.archive.is_null() {
                libarchive2_sys::archive_read_close(self.archive);
                libarchive2_sys::archive_read_free(self.archive);
            }
            // Clean up callback data if present
            // SAFETY: The callback data is only accessed once here. We take ownership
            // by taking from the Option, preventing double-free.
            if let Some((data, drop_fn)) = self._callback_data.take() {
                drop_fn(data);
            }
        }
    }
}

// Note: Default implementation removed because archive creation can fail.
// Use ReadArchive::new() instead.
