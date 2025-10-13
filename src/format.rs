//! Archive and compression format definitions

/// Archive format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// TAR format
    Tar,
    /// GNU TAR format with extensions
    TarGnu,
    /// PAX (POSIX TAR) format
    TarPax,
    /// Restricted PAX format
    TarPaxRestricted,
    /// POSIX ustar format
    TarUstar,
    /// ZIP format
    Zip,
    /// 7-Zip format
    SevenZip,
    /// AR (Unix archive) format
    Ar,
    /// CPIO format
    Cpio,
    /// ISO 9660 CD-ROM format
    Iso9660,
    /// XAR format
    Xar,
    /// MTREE format
    Mtree,
    /// RAW format (no formatting)
    Raw,
    /// Shar shell archive format
    Shar,
    /// WARC web archive format
    Warc,
    /// RAR format (read-only)
    Rar,
    /// RAR 5.x format (read-only)
    Rar5,
    /// LHA format (read-only)
    Lha,
    /// CAB format (read-only)
    Cab,
}

/// Compression format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    /// No compression
    None,
    /// Gzip compression
    Gzip,
    /// Bzip2 compression
    Bzip2,
    /// LZMA/XZ compression
    Xz,
    /// Zstd compression
    Zstd,
    /// LZ4 compression
    Lz4,
    /// Compress (LZW) compression
    Compress,
    /// UUEncode compression
    UuEncode,
    /// LZIP compression
    Lzip,
    /// LRZIP compression
    Lrzip,
    /// LZOP compression
    Lzop,
    /// GRZIP compression
    Grzip,
}

impl ArchiveFormat {
    /// Get the typical file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ArchiveFormat::Tar => "tar",
            ArchiveFormat::TarGnu => "tar",
            ArchiveFormat::TarPax => "tar",
            ArchiveFormat::TarPaxRestricted => "tar",
            ArchiveFormat::TarUstar => "tar",
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::SevenZip => "7z",
            ArchiveFormat::Ar => "ar",
            ArchiveFormat::Cpio => "cpio",
            ArchiveFormat::Iso9660 => "iso",
            ArchiveFormat::Xar => "xar",
            ArchiveFormat::Mtree => "mtree",
            ArchiveFormat::Raw => "bin",
            ArchiveFormat::Shar => "shar",
            ArchiveFormat::Warc => "warc",
            ArchiveFormat::Rar => "rar",
            ArchiveFormat::Rar5 => "rar",
            ArchiveFormat::Lha => "lha",
            ArchiveFormat::Cab => "cab",
        }
    }
}

impl CompressionFormat {
    /// Get the typical file extension for this compression format
    pub fn extension(&self) -> &'static str {
        match self {
            CompressionFormat::None => "",
            CompressionFormat::Gzip => "gz",
            CompressionFormat::Bzip2 => "bz2",
            CompressionFormat::Xz => "xz",
            CompressionFormat::Zstd => "zst",
            CompressionFormat::Lz4 => "lz4",
            CompressionFormat::Compress => "Z",
            CompressionFormat::UuEncode => "uu",
            CompressionFormat::Lzip => "lz",
            CompressionFormat::Lrzip => "lrz",
            CompressionFormat::Lzop => "lzo",
            CompressionFormat::Grzip => "grz",
        }
    }
}

/// Format specifier for reading archives
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadFormat {
    /// Auto-detect the format
    All,
    /// Specific format
    Format(ArchiveFormat),
}

/// ZIP compression method options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZipCompressionMethod {
    /// Store (no compression)
    Store,
    /// Deflate compression (default)
    Deflate,
}

/// Compression level (0-9)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressionLevel(u8);

impl CompressionLevel {
    /// Create a new compression level (0-9)
    ///
    /// # Panics
    /// Panics if level > 9
    pub fn new(level: u8) -> Self {
        assert!(level <= 9, "Compression level must be 0-9");
        CompressionLevel(level)
    }

    /// No compression (level 0)
    pub const NONE: Self = CompressionLevel(0);

    /// Fastest compression (level 1)
    pub const FASTEST: Self = CompressionLevel(1);

    /// Default/balanced compression (level 6)
    pub const DEFAULT: Self = CompressionLevel(6);

    /// Best compression (level 9)
    pub const BEST: Self = CompressionLevel(9);

    /// Get the numeric level value
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Format-specific options for archive writing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatOption {
    /// ZIP: Set compression method
    ZipCompressionMethod(ZipCompressionMethod),

    /// ZIP: Set compression level (0-9)
    ZipCompressionLevel(CompressionLevel),

    /// ISO9660: Set volume ID
    Iso9660VolumeId(String),

    /// ISO9660: Set publisher
    Iso9660Publisher(String),

    /// ISO9660: Allow lowercase filenames
    Iso9660AllowLowercase(bool),

    /// TAR: Use GNU extensions for long pathnames
    TarGnuLongPathnames(bool),

    /// 7z: Set compression level (0-9)
    SevenZipCompressionLevel(CompressionLevel),
}

/// Filter-specific options for compression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOption {
    /// Gzip: Set compression level (0-9)
    GzipCompressionLevel(CompressionLevel),

    /// Bzip2: Set compression level (0-9)
    Bzip2CompressionLevel(CompressionLevel),

    /// XZ: Set compression level (0-9)
    XzCompressionLevel(CompressionLevel),

    /// Zstd: Set compression level (0-22, but typically 0-9)
    ZstdCompressionLevel(u8),

    /// LZ4: Set compression level (0-9)
    Lz4CompressionLevel(CompressionLevel),
}
