//! Archive writing functionality

use crate::entry::{EntryMut, FileType};
use crate::error::{Error, Result};
use crate::format::{ArchiveFormat, CompressionFormat};
use std::ffi::CString;
use std::path::Path;
use std::time::SystemTime;

/// Archive writer with builder pattern and RAII resource management
pub struct WriteArchive {
    archive: *mut libarchive2_sys::archive,
    format: Option<ArchiveFormat>,
    compression: Option<CompressionFormat>,
}

impl WriteArchive {
    /// Create a new archive writer builder
    pub fn new() -> Self {
        WriteArchive {
            archive: std::ptr::null_mut(),
            format: None,
            compression: None,
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

    /// Open a file for writing
    pub fn open_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        unsafe {
            self.archive = libarchive2_sys::archive_write_new();
            if self.archive.is_null() {
                return Err(Error::NullPointer);
            }

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

    /// Write an entry header
    pub fn write_header(&mut self, entry: &EntryMut) -> Result<()> {
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

    /// Add a file to the archive
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P, data: &[u8]) -> Result<()> {
        let mut entry = EntryMut::new();
        entry.set_pathname(path)?;
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(data.len() as i64);
        entry.set_perm(0o644);
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
        entry.set_perm(0o755);
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
        }
        Ok(())
    }
}

impl Drop for WriteArchive {
    fn drop(&mut self) {
        unsafe {
            if !self.archive.is_null() {
                libarchive2_sys::archive_write_close(self.archive);
                libarchive2_sys::archive_write_free(self.archive);
            }
        }
    }
}

impl Default for WriteArchive {
    fn default() -> Self {
        Self::new()
    }
}
