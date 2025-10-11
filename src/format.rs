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
