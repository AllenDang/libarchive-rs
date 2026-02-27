//! Archive writing functionality

use crate::entry::{EntryMut, FileType};
use crate::error::{Error, Result};
use crate::format::{ArchiveFormat, CompressionFormat, FilterOption, FormatOption};
use std::ffi::CString;
use std::path::Path;
use std::time::SystemTime;

/// Archive writer with builder pattern and RAII resource management
///
/// The lifetime parameter 'a represents borrowed data (e.g., when writing to memory).
/// For file-based archives, use 'static lifetime.
///
/// # Thread Safety
///
/// `WriteArchive` is `Send` but not `Sync`. You can transfer ownership between threads,
/// but cannot share references across threads. This matches libarchive's thread safety
/// guarantees: archive objects should not be shared between threads, but can be moved.
pub struct WriteArchive<'a> {
    archive: *mut libarchive2_sys::archive,
    format: Option<ArchiveFormat>,
    compression: Option<CompressionFormat>,
    passphrase: Option<String>,
    format_options: Vec<FormatOption>,
    filter_options: Vec<FilterOption>,
    _callback_data: Option<(*mut std::ffi::c_void, crate::callbacks::DropFn)>,
    _phantom: std::marker::PhantomData<&'a mut [u8]>,
}

// SAFETY: WriteArchive can be sent between threads because:
// 1. The archive pointer is owned exclusively by this instance
// 2. libarchive archive objects can be used from different threads (just not concurrently)
// 3. All other fields (format, compression, passphrase, callback_data) are Send
// 4. The phantom data only tracks lifetimes, not actual data
unsafe impl<'a> Send for WriteArchive<'a> {}

// Note: WriteArchive is NOT Sync because libarchive archives are not thread-safe
// for concurrent access.

impl<'a> Default for WriteArchive<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> WriteArchive<'a> {
    /// Create a new archive writer builder
    pub fn new() -> Self {
        WriteArchive {
            archive: std::ptr::null_mut(),
            format: None,
            compression: None,
            passphrase: None,
            format_options: Vec::new(),
            filter_options: Vec::new(),
            _callback_data: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the archive format
    pub fn format(mut self, format: ArchiveFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the compression format
    pub fn compression(mut self, compression: CompressionFormat) -> Self {
        self.compression = Some(compression);
        self
    }

    /// Set a passphrase for encryption (ZIP and 7z formats)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::{WriteArchive, ArchiveFormat};
    ///
    /// let mut archive = WriteArchive::new()
    ///     .format(ArchiveFormat::Zip)
    ///     .passphrase("my_password")
    ///     .open_file("encrypted.zip")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn passphrase<S: Into<String>>(mut self, passphrase: S) -> Self {
        self.passphrase = Some(passphrase.into());
        self
    }

    /// Set a format-specific option
    ///
    /// This allows fine-grained control over format-specific behavior such as
    /// compression levels, metadata, or format features.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::{WriteArchive, ArchiveFormat, FormatOption, CompressionLevel, ZipCompressionMethod};
    ///
    /// let mut archive = WriteArchive::new()
    ///     .format(ArchiveFormat::Zip)
    ///     .format_option(FormatOption::ZipCompressionLevel(CompressionLevel::BEST))
    ///     .format_option(FormatOption::ZipCompressionMethod(ZipCompressionMethod::Deflate))
    ///     .open_file("output.zip")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn format_option(mut self, option: FormatOption) -> Self {
        self.format_options.push(option);
        self
    }

    /// Set a filter/compression-specific option
    ///
    /// This allows fine-grained control over compression behavior such as
    /// compression levels for different compression formats.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::{WriteArchive, ArchiveFormat, CompressionFormat, FilterOption, CompressionLevel};
    ///
    /// let mut archive = WriteArchive::new()
    ///     .format(ArchiveFormat::TarPax)
    ///     .compression(CompressionFormat::Gzip)
    ///     .filter_option(FilterOption::GzipCompressionLevel(CompressionLevel::BEST))
    ///     .open_file("output.tar.gz")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn filter_option(mut self, option: FilterOption) -> Self {
        self.filter_options.push(option);
        self
    }

    /// Open a file for writing
    pub fn open_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        unsafe {
            self.archive = libarchive2_sys::archive_write_new();
            if self.archive.is_null() {
                return Err(Error::NullPointer);
            }

            // Configure format and compression using helper
            self.configure_format_and_compression()?;

            // Open the file
            let path_str = path
                .as_ref()
                .to_str()
                .ok_or_else(|| Error::InvalidArgument("Path contains invalid UTF-8".to_string()))?;
            let c_path = CString::new(path_str)
                .map_err(|_| Error::InvalidArgument("Path contains null byte".to_string()))?;

            Error::from_return_code(
                libarchive2_sys::archive_write_open_filename(self.archive, c_path.as_ptr()),
                self.archive,
            )?;

            Ok(self)
        }
    }

    /// Open an in-memory archive
    ///
    /// The archive data will be written to the provided buffer.
    /// The buffer must remain valid for the lifetime of the WriteArchive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::{WriteArchive, ArchiveFormat, CompressionFormat};
    ///
    /// let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer
    /// let mut used = 0;
    ///
    /// let mut archive = WriteArchive::new()
    ///     .format(ArchiveFormat::Zip)
    ///     .open_memory(&mut buffer, &mut used)?;
    ///
    /// archive.add_file("test.txt", b"Hello, world!")?;
    /// archive.finish()?;
    ///
    /// println!("Archive size: {} bytes", used);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open_memory(mut self, buffer: &'a mut [u8], used: &'a mut usize) -> Result<Self> {
        unsafe {
            self.archive = libarchive2_sys::archive_write_new();
            if self.archive.is_null() {
                return Err(Error::NullPointer);
            }

            // Configure format and compression using helper
            self.configure_format_and_compression()?;

            // Open memory
            // SAFETY: The buffer and used pointers are tied to lifetime 'a via PhantomData,
            // ensuring they remain valid for the lifetime of the WriteArchive.
            Error::from_return_code(
                libarchive2_sys::archive_write_open_memory(
                    self.archive,
                    buffer.as_mut_ptr() as *mut std::os::raw::c_void,
                    buffer.len(),
                    used as *mut usize,
                ),
                self.archive,
            )?;

            Ok(self)
        }
    }

    /// Open an archive for writing to a file descriptor
    ///
    /// # Safety
    /// The file descriptor must be valid and remain open for the lifetime of the archive.
    /// The archive will not close the file descriptor when dropped.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::{WriteArchive, ArchiveFormat};
    /// use std::fs::File;
    /// use std::os::unix::io::AsRawFd;
    ///
    /// let file = File::create("output.tar.gz")?;
    /// let mut archive = WriteArchive::new()
    ///     .format(ArchiveFormat::TarPax)
    ///     .open_fd(file.as_raw_fd())?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[cfg(unix)]
    pub fn open_fd(mut self, fd: std::os::unix::io::RawFd) -> Result<Self> {
        unsafe {
            self.archive = libarchive2_sys::archive_write_new();
            if self.archive.is_null() {
                return Err(Error::NullPointer);
            }

            self.configure_format_and_compression()?;

            Error::from_return_code(
                libarchive2_sys::archive_write_open_fd(self.archive, fd),
                self.archive,
            )?;

            Ok(self)
        }
    }

    /// Open an archive for writing to a file descriptor (Windows)
    ///
    /// # Safety
    /// The file descriptor must be valid and remain open for the lifetime of the archive.
    /// The archive will not close the file descriptor when dropped.
    #[cfg(windows)]
    pub fn open_fd(mut self, fd: std::os::windows::io::RawHandle) -> Result<Self> {
        unsafe {
            self.archive = libarchive2_sys::archive_write_new();
            if self.archive.is_null() {
                return Err(Error::NullPointer);
            }

            self.configure_format_and_compression()?;

            Error::from_return_code(
                libarchive2_sys::archive_write_open_fd(self.archive, fd as std::os::raw::c_int),
                self.archive,
            )?;

            Ok(self)
        }
    }

    /// Open an archive using a custom callback
    ///
    /// This allows writing archives to custom destinations like network streams,
    /// encrypted files, or any other destination that can be wrapped in a Write trait.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::{WriteArchive, CallbackWriter, ArchiveFormat};
    /// use std::fs::File;
    ///
    /// let file = File::create("output.tar.gz")?;
    /// let callback = CallbackWriter::new(file);
    /// let mut archive = WriteArchive::new()
    ///     .format(ArchiveFormat::TarPax)
    ///     .open_callback(callback)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open_callback<W: std::io::Write + 'static>(
        mut self,
        callback: crate::callbacks::CallbackWriter<W>,
    ) -> Result<Self> {
        unsafe {
            self.archive = libarchive2_sys::archive_write_new();
            if self.archive.is_null() {
                return Err(Error::NullPointer);
            }

            self.configure_format_and_compression()?;

            let (client_data, write_cb, close_cb, drop_fn) = callback.into_raw_parts();

            // SAFETY: The function pointers returned from into_raw_parts are guaranteed
            // to have the correct signatures for the libarchive callback functions.
            // We cast them from *const c_void back to their proper function pointer types.
            type WriteFn = unsafe extern "C" fn(
                *mut libarchive2_sys::archive,
                *mut std::ffi::c_void,
                *const std::ffi::c_void,
                usize,
            ) -> isize;
            type CloseFn = unsafe extern "C" fn(
                *mut libarchive2_sys::archive,
                *mut std::ffi::c_void,
            ) -> std::os::raw::c_int;

            // Cast from void pointer to function pointer
            let write_fn = Some(std::mem::transmute::<*const std::ffi::c_void, WriteFn>(
                write_cb,
            ));
            let close_fn = Some(std::mem::transmute::<*const std::ffi::c_void, CloseFn>(
                close_cb,
            ));

            Error::from_return_code(
                libarchive2_sys::archive_write_open(
                    self.archive,
                    client_data,
                    None,
                    write_fn,
                    close_fn,
                ),
                self.archive,
            )?;

            self._callback_data = Some((client_data, drop_fn));
            Ok(self)
        }
    }

    /// Configure format and compression (helper for open methods)
    fn configure_format_and_compression(&mut self) -> Result<()> {
        unsafe {
            // Set format
            match self.format.unwrap_or(ArchiveFormat::TarPax) {
                ArchiveFormat::Tar => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_pax(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::TarGnu => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_gnutar(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::TarPax | ArchiveFormat::TarPaxRestricted => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_pax(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::TarUstar => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_ustar(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Zip => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_zip(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::SevenZip => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_7zip(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Ar => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_ar_bsd(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Cpio => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_cpio(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Iso9660 => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_iso9660(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Xar => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_xar(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Mtree => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_mtree(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Raw => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_raw(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Shar => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_shar(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Warc => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_warc(self.archive),
                        self.archive,
                    )?;
                }
                ArchiveFormat::Rar
                | ArchiveFormat::Rar5
                | ArchiveFormat::Lha
                | ArchiveFormat::Cab => {
                    return Err(Error::InvalidArgument(format!(
                        "Format {:?} is read-only and cannot be used for writing",
                        self.format
                    )));
                }
            }

            // Set compression
            match self.compression.unwrap_or(CompressionFormat::None) {
                CompressionFormat::None => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_none(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::Gzip => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_gzip(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::Bzip2 => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_bzip2(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::Xz => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_xz(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::Zstd => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_zstd(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::Lz4 => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_lz4(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::Compress => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_compress(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::UuEncode => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_uuencode(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::Lrzip => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_lrzip(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::Lzop => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_lzop(self.archive),
                        self.archive,
                    )?;
                }
                CompressionFormat::Grzip => {
                    Error::from_return_code(
                        libarchive2_sys::archive_write_add_filter_grzip(self.archive),
                        self.archive,
                    )?;
                }
                _ => {
                    return Err(Error::InvalidArgument(format!(
                        "Unsupported compression: {:?}",
                        self.compression
                    )));
                }
            }

            // Set passphrase if provided
            if let Some(ref passphrase) = self.passphrase {
                let c_passphrase = CString::new(passphrase.as_str()).map_err(|_| {
                    Error::InvalidArgument("Passphrase contains null byte".to_string())
                })?;
                Error::from_return_code(
                    libarchive2_sys::archive_write_set_passphrase(
                        self.archive,
                        c_passphrase.as_ptr(),
                    ),
                    self.archive,
                )?;
            }

            // Apply format options
            for option in &self.format_options {
                self.apply_format_option(option)?;
            }

            // Apply filter options
            for option in &self.filter_options {
                self.apply_filter_option(option)?;
            }

            Ok(())
        }
    }

    /// Apply a format-specific option (internal helper)
    fn apply_format_option(&self, option: &FormatOption) -> Result<()> {
        use crate::format::ZipCompressionMethod;

        unsafe {
            match option {
                FormatOption::ZipCompressionMethod(method) => {
                    let method_str = match method {
                        ZipCompressionMethod::Store => CString::new("store").unwrap(),
                        ZipCompressionMethod::Deflate => CString::new("deflate").unwrap(),
                    };
                    let module = CString::new("zip").unwrap();
                    let key = CString::new("compression").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            method_str.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FormatOption::ZipCompressionLevel(level) => {
                    let level_str = CString::new(level.value().to_string()).unwrap();
                    let module = CString::new("zip").unwrap();
                    let key = CString::new("compression-level").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            level_str.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FormatOption::Iso9660VolumeId(volume_id) => {
                    let vol_id = CString::new(volume_id.as_str()).map_err(|_| {
                        Error::InvalidArgument("Volume ID contains null byte".to_string())
                    })?;
                    let module = CString::new("iso9660").unwrap();
                    let key = CString::new("volume-id").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            vol_id.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FormatOption::Iso9660Publisher(publisher) => {
                    let pub_str = CString::new(publisher.as_str()).map_err(|_| {
                        Error::InvalidArgument("Publisher contains null byte".to_string())
                    })?;
                    let module = CString::new("iso9660").unwrap();
                    let key = CString::new("publisher").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            pub_str.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FormatOption::Iso9660AllowLowercase(allow) => {
                    let val = CString::new(if *allow { "1" } else { "0" }).unwrap();
                    let module = CString::new("iso9660").unwrap();
                    let key = CString::new("allow-lowercase").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            val.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FormatOption::TarGnuLongPathnames(enable) => {
                    let val = CString::new(if *enable { "1" } else { "0" }).unwrap();
                    let module = CString::new("gnutar").unwrap();
                    let key = CString::new("longname").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            val.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FormatOption::SevenZipCompressionLevel(level) => {
                    let level_str = CString::new(level.value().to_string()).unwrap();
                    let module = CString::new("7zip").unwrap();
                    let key = CString::new("compression-level").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_format_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            level_str.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
            }
            Ok(())
        }
    }

    /// Apply a filter-specific option (internal helper)
    fn apply_filter_option(&self, option: &FilterOption) -> Result<()> {
        unsafe {
            match option {
                FilterOption::GzipCompressionLevel(level) => {
                    let level_str = CString::new(level.value().to_string()).unwrap();
                    let module = CString::new("gzip").unwrap();
                    let key = CString::new("compression-level").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_filter_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            level_str.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FilterOption::Bzip2CompressionLevel(level) => {
                    let level_str = CString::new(level.value().to_string()).unwrap();
                    let module = CString::new("bzip2").unwrap();
                    let key = CString::new("compression-level").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_filter_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            level_str.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FilterOption::XzCompressionLevel(level) => {
                    let level_str = CString::new(level.value().to_string()).unwrap();
                    let module = CString::new("xz").unwrap();
                    let key = CString::new("compression-level").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_filter_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            level_str.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FilterOption::ZstdCompressionLevel(level) => {
                    let level_str = CString::new(level.to_string()).unwrap();
                    let module = CString::new("zstd").unwrap();
                    let key = CString::new("compression-level").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_filter_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            level_str.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
                FilterOption::Lz4CompressionLevel(level) => {
                    let level_str = CString::new(level.value().to_string()).unwrap();
                    let module = CString::new("lz4").unwrap();
                    let key = CString::new("compression-level").unwrap();
                    Error::from_return_code(
                        libarchive2_sys::archive_write_set_filter_option(
                            self.archive,
                            module.as_ptr(),
                            key.as_ptr(),
                            level_str.as_ptr(),
                        ),
                        self.archive,
                    )?;
                }
            }
            Ok(())
        }
    }

    /// Write an entry header
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

    /// Write a data block at a specific offset
    ///
    /// This is useful for sparse files where you want to write data at specific
    /// offsets, leaving holes (zeros) in between. This method provides fine-grained
    /// control over data placement within an entry.
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset within the entry where data should be written
    /// * `data` - Data to write at the specified offset
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written on success.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::{WriteArchive, ArchiveFormat, EntryMut, FileType};
    ///
    /// let mut archive = WriteArchive::new()
    ///     .format(ArchiveFormat::TarPax)
    ///     .open_file("sparse.tar")?;
    ///
    /// // Create a sparse file with data at specific offsets
    /// let mut entry = EntryMut::new();
    /// entry.set_pathname("sparse_file.bin")?;
    /// entry.set_file_type(FileType::RegularFile);
    /// entry.set_size(1024 * 1024); // 1MB file
    /// entry.set_perm(0o644)?;
    ///
    /// archive.write_header(&entry)?;
    ///
    /// // Write data at offset 0
    /// archive.write_data_block(0, b"Start of file")?;
    ///
    /// // Skip to offset 512KB and write more data (creating a hole)
    /// archive.write_data_block(512 * 1024, b"Middle of file")?;
    ///
    /// // Skip to offset 1MB-100 and write end data
    /// archive.write_data_block(1024 * 1024 - 100, b"End of file")?;
    ///
    /// archive.finish()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Notes
    ///
    /// - The offset determines where in the entry the data will be written
    /// - Gaps between writes are typically represented as sparse holes (zeros) in the archive
    /// - Not all archive formats support sparse files (e.g., TAR formats do, but ZIP does not)
    /// - The entry's size must be set appropriately before writing blocks
    pub fn write_data_block(&mut self, offset: i64, data: &[u8]) -> Result<usize> {
        unsafe {
            let ret = libarchive2_sys::archive_write_data_block(
                self.archive,
                data.as_ptr() as *const std::os::raw::c_void,
                data.len(),
                offset,
            );

            if ret < 0 {
                Err(Error::from_archive(self.archive))
            } else {
                // archive_write_data_block returns ARCHIVE_OK (0) on success
                // We return the number of bytes written (data.len())
                Ok(data.len())
            }
        }
    }

    /// Add a file to the archive
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P, data: &[u8]) -> Result<()> {
        let mut entry = EntryMut::new();
        entry.set_pathname(path)?;
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(data.len() as i64);
        entry.set_perm(0o644)?;
        entry.set_mtime(SystemTime::now());

        self.write_header(&entry)?;
        self.write_data(data)?;

        Ok(())
    }

    /// Add a directory to the archive
    pub fn add_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let mut entry = EntryMut::new();
        entry.set_pathname(path)?;
        entry.set_file_type(FileType::Directory);
        entry.set_perm(0o755)?;
        entry.set_mtime(SystemTime::now());

        self.write_header(&entry)?;

        Ok(())
    }

    /// Finish writing and close the archive
    pub fn finish(mut self) -> Result<()> {
        unsafe {
            if !self.archive.is_null() {
                Error::from_return_code(
                    libarchive2_sys::archive_write_close(self.archive),
                    self.archive,
                )?;
                libarchive2_sys::archive_write_free(self.archive);
                self.archive = std::ptr::null_mut();
            }
            // Clean up callback data now to prevent double-free in Drop
            // SAFETY: We take ownership from the Option, so Drop won't access it again
            if let Some((data, drop_fn)) = self._callback_data.take() {
                drop_fn(data);
            }
        }
        Ok(())
    }
}

/// `std::io::Write` implementation for writing data to the current archive entry.
///
/// This allows using `WriteArchive` with anything that accepts a `Write` trait object,
/// such as `io::copy()`. You must call [`write_header()`](WriteArchive::write_header)
/// before writing data.
///
/// # Examples
///
/// ```no_run
/// use std::io::Write;
/// use libarchive2::{WriteArchive, ArchiveFormat, EntryMut, FileType};
///
/// let mut archive = WriteArchive::new()
///     .format(ArchiveFormat::TarPax)
///     .open_file("output.tar")?;
///
/// let mut entry = EntryMut::new();
/// entry.set_pathname("hello.txt")?;
/// entry.set_file_type(FileType::RegularFile);
/// entry.set_size(13);
/// entry.set_perm(0o644)?;
/// archive.write_header(&entry)?;
///
/// // Use std::io::Write trait
/// archive.write_all(b"Hello, world!")?;
///
/// archive.finish()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ## Composing with other writers
///
/// Use [`CallbackWriter`](crate::CallbackWriter) to pipe archive output through
/// any `std::io::Write` destination (e.g., a compressor, network socket, or buffer):
///
/// ```no_run
/// use libarchive2::{WriteArchive, CallbackWriter, ArchiveFormat};
///
/// let output_file = std::fs::File::create("archive.tar")?;
/// // Could wrap output_file in flate2::write::GzEncoder here for extra compression
/// let callback = CallbackWriter::new(output_file);
/// let mut archive = WriteArchive::new()
///     .format(ArchiveFormat::TarPax)
///     .open_callback(callback)?;
///
/// archive.add_file("file.txt", b"content")?;
/// archive.finish()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
impl<'a> std::io::Write for WriteArchive<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_data(buf).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // libarchive manages its own buffering; there is no flush operation.
        Ok(())
    }
}

impl<'a> Drop for WriteArchive<'a> {
    fn drop(&mut self) {
        unsafe {
            if !self.archive.is_null() {
                libarchive2_sys::archive_write_close(self.archive);
                libarchive2_sys::archive_write_free(self.archive);
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

// Note: Default implementation removed for consistency with ReadArchive.
// Use WriteArchive::new() instead.
