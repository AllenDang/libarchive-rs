//! Archive reading functionality

use crate::entry::Entry;
use crate::error::{Error, Result};
use crate::format::{CompressionFormat, ReadFormat};
use std::ffi::CString;
use std::path::Path;
use std::ptr;

/// Archive reader with RAII resource management
pub struct ReadArchive {
    archive: *mut libarchive2_sys::archive,
}

impl ReadArchive {
    /// Create a new archive reader
    pub fn new() -> Result<Self> {
        unsafe {
            let archive = libarchive2_sys::archive_read_new();
            if archive.is_null() {
                return Err(Error::NullPointer);
            }
            Ok(ReadArchive { archive })
        }
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

    /// Open an archive from memory
    pub fn open_memory(data: &[u8]) -> Result<Self> {
        let mut reader = Self::new()?;
        reader.support_filter_all()?;
        reader.support_format_all()?;

        unsafe {
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
                        ArchiveFormat::Cpio => {
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

    /// Read the next entry header
    ///
    /// Returns `None` when there are no more entries
    pub fn next_entry(&mut self) -> Result<Option<Entry<'_>>> {
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
}

impl Drop for ReadArchive {
    fn drop(&mut self) {
        unsafe {
            if !self.archive.is_null() {
                libarchive2_sys::archive_read_close(self.archive);
                libarchive2_sys::archive_read_free(self.archive);
            }
        }
    }
}

impl Default for ReadArchive {
    fn default() -> Self {
        Self::new().expect("Failed to create ReadArchive")
    }
}
