//! macOS `.pkg` installer file reading and writing
//!
//! macOS `.pkg` files are XAR archives containing a `Payload` file that is
//! pbzx-compressed CPIO data. This module provides [`PkgReader`] for reading
//! and [`PkgWriter`] for creating these files.
//!
//! # Examples
//!
//! ## Reading a .pkg file
//!
//! ```no_run
//! use libarchive2::PkgReader;
//!
//! let mut pkg = PkgReader::open("installer.pkg")?;
//! while let Some(entry) = pkg.next_entry()? {
//!     println!("{}", entry.pathname().unwrap_or_default());
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Writing a .pkg file
//!
//! ```no_run
//! use libarchive2::PkgWriter;
//!
//! let mut pkg = PkgWriter::new();
//! pkg.add_file("usr/local/bin/hello", b"#!/bin/sh\necho hello\n")?;
//! pkg.add_directory("usr/local/share/myapp")?;
//! pkg.write("output.pkg")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::entry::{Entry, EntryMut, FileType};
use crate::error::{Error, Result};
use crate::format::{ArchiveFormat, CompressionFormat, ReadFormat};
use crate::reader::ReadArchive;
use crate::writer::WriteArchive;
use std::path::Path;
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// PkgReader
// ---------------------------------------------------------------------------

/// Reader for macOS `.pkg` installer files
///
/// Automatically handles the XAR -> pbzx -> CPIO chain so you can iterate
/// over the installed files directly.
///
/// # Examples
///
/// ```no_run
/// use libarchive2::PkgReader;
///
/// let mut pkg = PkgReader::open("MyApp.pkg")?;
/// while let Some(entry) = pkg.next_entry()? {
///     let path = entry.pathname().unwrap_or_default();
///     let size = entry.size();
///     println!("{path} ({size} bytes)");
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct PkgReader {
    /// Decompressed CPIO data (owned so the inner ReadArchive can borrow it)
    _cpio_data: Vec<u8>,
    /// Inner archive reader over the CPIO data.
    /// Raw pointer to manage drop order: the reader must be dropped before _cpio_data.
    inner: *mut ReadArchive<'static>,
}

// SAFETY: PkgReader owns both the data and the reader exclusively.
// Same Send-but-not-Sync contract as ReadArchive.
unsafe impl Send for PkgReader {}

impl PkgReader {
    /// Open a `.pkg` file and prepare it for reading
    ///
    /// This extracts the Payload from the XAR container, decompresses the
    /// pbzx stream, and opens the resulting CPIO archive for iteration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be opened as a XAR archive
    /// - No `Payload` entry is found inside the XAR
    /// - The Payload is not valid pbzx data
    /// - The decompressed content is not a valid CPIO archive
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let payload_data = Self::extract_payload(path)?;
        let cpio_data = crate::pbzx::decompress(&payload_data)?;
        Self::from_cpio_data(cpio_data)
    }

    /// Create a `PkgReader` from raw pbzx-compressed data
    ///
    /// Use this when you already have the Payload bytes (e.g., extracted
    /// from a XAR archive manually).
    pub fn from_pbzx(data: &[u8]) -> Result<Self> {
        let cpio_data = crate::pbzx::decompress(data)?;
        Self::from_cpio_data(cpio_data)
    }

    /// Create a `PkgReader` from already-decompressed CPIO data
    pub fn from_cpio(cpio_data: Vec<u8>) -> Result<Self> {
        Self::from_cpio_data(cpio_data)
    }

    /// Read the next entry from the package
    ///
    /// Returns `None` when all entries have been read.
    pub fn next_entry(&mut self) -> Result<Option<Entry<'_>>> {
        // SAFETY: inner is valid and points to a ReadArchive that borrows _cpio_data.
        // We ensure proper lifetime management through the struct's drop order.
        unsafe { &mut *self.inner }.next_entry()
    }

    /// Read data from the current entry into the provided buffer
    ///
    /// Returns the number of bytes read (0 means end of entry data).
    pub fn read_data(&mut self, buf: &mut [u8]) -> Result<usize> {
        unsafe { &mut *self.inner }.read_data(buf)
    }

    /// Read all data from the current entry into a vector
    pub fn read_data_to_vec(&mut self) -> Result<Vec<u8>> {
        unsafe { &mut *self.inner }.read_data_to_vec()
    }

    /// Skip the data for the current entry
    pub fn skip_data(&mut self) -> Result<()> {
        unsafe { &mut *self.inner }.skip_data()
    }

    // -- internal helpers --

    fn extract_payload<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
        let mut xar = ReadArchive::new()?;
        xar.support_filter_all()?;
        xar.support_format(ReadFormat::Format(ArchiveFormat::Xar))?;

        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::InvalidArgument("Path contains invalid UTF-8".to_string()))?;
        let c_path = std::ffi::CString::new(path_str)
            .map_err(|_| Error::InvalidArgument("Path contains null byte".to_string()))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_open_filename(xar.archive(), c_path.as_ptr(), 10240),
                xar.archive(),
            )?;
        }

        while let Some(entry) = xar.next_entry()? {
            let name = entry.pathname().unwrap_or_default();
            if name == "Payload" || name.ends_with("/Payload") {
                return xar.read_data_to_vec();
            }
        }

        Err(Error::InvalidArgument(
            "No Payload entry found in .pkg file".to_string(),
        ))
    }

    fn from_cpio_data(cpio_data: Vec<u8>) -> Result<Self> {
        let cpio_ptr = cpio_data.as_ptr();
        let cpio_len = cpio_data.len();

        let mut reader = ReadArchive::new()?;
        reader.support_filter_all()?;
        reader.support_format(ReadFormat::Format(ArchiveFormat::Cpio))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_read_open_memory(
                    reader.archive(),
                    cpio_ptr as *const std::os::raw::c_void,
                    cpio_len,
                ),
                reader.archive(),
            )?;
        }

        // Transmute the lifetime to 'static. This is safe because:
        // 1. _cpio_data is owned by PkgReader and won't be dropped before inner
        // 2. We ensure proper drop order in the Drop impl
        let reader: ReadArchive<'static> = unsafe { std::mem::transmute(reader) };
        let inner = Box::into_raw(Box::new(reader));

        Ok(PkgReader {
            _cpio_data: cpio_data,
            inner,
        })
    }
}

impl Drop for PkgReader {
    fn drop(&mut self) {
        // Drop the reader first (it borrows _cpio_data), then _cpio_data
        // is dropped automatically by the compiler.
        unsafe {
            drop(Box::from_raw(self.inner));
        }
    }
}

// ---------------------------------------------------------------------------
// PkgWriter
// ---------------------------------------------------------------------------

/// Writer for macOS `.pkg` installer files
///
/// Collects files and directories, then produces a `.pkg` file
/// (XAR archive containing a pbzx-compressed CPIO Payload).
///
/// # Examples
///
/// ```no_run
/// use libarchive2::PkgWriter;
///
/// let mut pkg = PkgWriter::new();
/// pkg.add_file("usr/local/bin/hello", b"#!/bin/sh\necho hello\n")?;
/// pkg.add_directory("usr/local/share/myapp")?;
/// pkg.write("output.pkg")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct PkgWriter {
    entries: Vec<PkgEntry>,
}

struct PkgEntry {
    entry: EntryMut,
    data: Vec<u8>,
}

impl PkgWriter {
    /// Create a new package writer
    pub fn new() -> Self {
        PkgWriter {
            entries: Vec::new(),
        }
    }

    /// Add a regular file to the package
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the file inside the package
    /// * `data` - The file contents
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P, data: &[u8]) -> Result<()> {
        let mut entry = EntryMut::new();
        entry.set_pathname(path)?;
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(data.len() as i64);
        entry.set_perm(0o644)?;
        entry.set_mtime(SystemTime::now());

        self.entries.push(PkgEntry {
            entry,
            data: data.to_vec(),
        });
        Ok(())
    }

    /// Add a regular file with custom permissions
    pub fn add_file_with_perm<P: AsRef<Path>>(
        &mut self,
        path: P,
        data: &[u8],
        perm: u32,
    ) -> Result<()> {
        let mut entry = EntryMut::new();
        entry.set_pathname(path)?;
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(data.len() as i64);
        entry.set_perm(perm)?;
        entry.set_mtime(SystemTime::now());

        self.entries.push(PkgEntry {
            entry,
            data: data.to_vec(),
        });
        Ok(())
    }

    /// Add a directory to the package
    pub fn add_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let mut entry = EntryMut::new();
        entry.set_pathname(path)?;
        entry.set_file_type(FileType::Directory);
        entry.set_size(0);
        entry.set_perm(0o755)?;
        entry.set_mtime(SystemTime::now());

        self.entries.push(PkgEntry {
            entry,
            data: Vec::new(),
        });
        Ok(())
    }

    /// Add a symlink to the package
    pub fn add_symlink<P: AsRef<Path>>(&mut self, path: P, target: &str) -> Result<()> {
        let mut entry = EntryMut::new();
        entry.set_pathname(path)?;
        entry.set_file_type(FileType::SymbolicLink);
        entry.set_symlink(target)?;
        entry.set_size(0);
        entry.set_perm(0o777)?;
        entry.set_mtime(SystemTime::now());

        self.entries.push(PkgEntry {
            entry,
            data: Vec::new(),
        });
        Ok(())
    }

    /// Write the package to a file
    ///
    /// This performs the full CPIO -> pbzx -> XAR pipeline.
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let pbzx_payload = self.build_pbzx_payload()?;
        self.write_xar(path, &pbzx_payload)
    }

    /// Build the package as in-memory bytes
    ///
    /// Returns the raw XAR archive bytes containing the pbzx-compressed CPIO Payload.
    pub fn write_to_vec(&self) -> Result<Vec<u8>> {
        let pbzx_payload = self.build_pbzx_payload()?;
        self.build_xar_bytes(&pbzx_payload)
    }

    // -- internal helpers --

    fn build_cpio(&self) -> Result<Vec<u8>> {
        // Write CPIO to memory
        let mut buffer = vec![0u8; 64 * 1024 * 1024]; // 64 MiB max
        let mut used = 0usize;

        {
            let mut archive = WriteArchive::new()
                .format(ArchiveFormat::Cpio)
                .compression(CompressionFormat::None)
                .open_memory(&mut buffer, &mut used)?;

            for pkg_entry in &self.entries {
                archive.write_header(&pkg_entry.entry)?;
                if !pkg_entry.data.is_empty() {
                    archive.write_data(&pkg_entry.data)?;
                }
            }

            archive.finish()?;
        }

        Ok(buffer[..used].to_vec())
    }

    fn build_pbzx_payload(&self) -> Result<Vec<u8>> {
        let cpio_data = self.build_cpio()?;
        crate::pbzx::compress(&cpio_data)
    }

    fn write_xar<P: AsRef<Path>>(&self, path: P, payload: &[u8]) -> Result<()> {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::Xar)
            .compression(CompressionFormat::None)
            .open_file(path)?;

        let mut entry = EntryMut::new();
        entry.set_pathname("Payload")?;
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(payload.len() as i64);
        entry.set_perm(0o644)?;
        entry.set_mtime(SystemTime::now());

        archive.write_header(&entry)?;
        archive.write_data(payload)?;
        archive.finish()?;
        Ok(())
    }

    fn build_xar_bytes(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; payload.len() + 4096]; // payload + XAR overhead
        let mut used = 0usize;

        {
            let mut archive = WriteArchive::new()
                .format(ArchiveFormat::Xar)
                .compression(CompressionFormat::None)
                .open_memory(&mut buffer, &mut used)?;

            let mut entry = EntryMut::new();
            entry.set_pathname("Payload")?;
            entry.set_file_type(FileType::RegularFile);
            entry.set_size(payload.len() as i64);
            entry.set_perm(0o644)?;
            entry.set_mtime(SystemTime::now());

            archive.write_header(&entry)?;
            archive.write_data(payload)?;
            archive.finish()?;
        }

        Ok(buffer[..used].to_vec())
    }
}

impl Default for PkgWriter {
    fn default() -> Self {
        Self::new()
    }
}
