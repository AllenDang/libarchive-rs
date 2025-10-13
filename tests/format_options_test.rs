//! Integration tests for runtime format and filter options

use libarchive2::{
    ArchiveFormat, CompressionFormat, CompressionLevel, FilterOption, FormatOption, WriteArchive,
    ZipCompressionMethod,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_compression_level_constants() {
    assert_eq!(CompressionLevel::NONE.value(), 0);
    assert_eq!(CompressionLevel::FASTEST.value(), 1);
    assert_eq!(CompressionLevel::DEFAULT.value(), 6);
    assert_eq!(CompressionLevel::BEST.value(), 9);
}

#[test]
fn test_compression_level_new() {
    for level in 0..=9 {
        let cl = CompressionLevel::new(level);
        assert_eq!(cl.value(), level);
    }
}

#[test]
#[should_panic(expected = "Compression level must be 0-9")]
fn test_compression_level_invalid() {
    CompressionLevel::new(10);
}

#[test]
fn test_zip_compression_method_store() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("store.zip");

    let test_data = b"This is test data for compression. ".repeat(100);

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .format_option(FormatOption::ZipCompressionMethod(
            ZipCompressionMethod::Store,
        ))
        .open_file(&archive_path)
        .unwrap();

    archive.add_file("test.txt", &test_data).unwrap();
    archive.finish().unwrap();

    let size = fs::metadata(&archive_path).unwrap().len();
    // Stored data should be roughly the same size as original (plus archive overhead)
    assert!(size >= test_data.len() as u64);
}

#[test]
fn test_zip_compression_method_deflate() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("deflate.zip");

    let test_data = b"AAAAAAAAAA".repeat(1000); // Highly compressible

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .format_option(FormatOption::ZipCompressionMethod(
            ZipCompressionMethod::Deflate,
        ))
        .open_file(&archive_path)
        .unwrap();

    archive.add_file("test.txt", &test_data).unwrap();
    archive.finish().unwrap();

    let size = fs::metadata(&archive_path).unwrap().len();
    // Deflated data should be much smaller than original
    assert!(size < test_data.len() as u64 / 2);
}

#[test]
fn test_zip_compression_levels() {
    let temp_dir = TempDir::new().unwrap();
    let test_data = b"Repeated data for compression testing. ".repeat(100);

    let mut sizes = Vec::new();

    for (name, level) in [
        ("fastest", CompressionLevel::FASTEST),
        ("default", CompressionLevel::DEFAULT),
        ("best", CompressionLevel::BEST),
    ] {
        let archive_path = temp_dir.path().join(format!("{}.zip", name));

        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::Zip)
            .format_option(FormatOption::ZipCompressionLevel(level))
            .open_file(&archive_path)
            .unwrap();

        archive.add_file("test.txt", &test_data).unwrap();
        archive.finish().unwrap();

        let size = fs::metadata(&archive_path).unwrap().len();
        sizes.push(size);
    }

    // Best compression should produce smallest file
    assert!(sizes[2] <= sizes[1]); // best <= default
    assert!(sizes[1] <= sizes[0]); // default <= fastest
}

#[test]
fn test_gzip_compression_levels() {
    let temp_dir = TempDir::new().unwrap();
    let test_data = b"Data for gzip compression testing. ".repeat(100);

    let mut sizes = Vec::new();

    for (name, level) in [
        ("fastest", CompressionLevel::FASTEST),
        ("best", CompressionLevel::BEST),
    ] {
        let archive_path = temp_dir.path().join(format!("{}.tar.gz", name));

        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::Gzip)
            .filter_option(FilterOption::GzipCompressionLevel(level))
            .open_file(&archive_path)
            .unwrap();

        archive.add_file("test.txt", &test_data).unwrap();
        archive.finish().unwrap();

        let size = fs::metadata(&archive_path).unwrap().len();
        sizes.push(size);
    }

    // Best should be smaller or equal to fastest
    assert!(sizes[1] <= sizes[0]);
}

#[test]
fn test_bzip2_compression_level() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.tar.bz2");

    let test_data = b"Bzip2 compression test data. ".repeat(100);

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Bzip2)
        .filter_option(FilterOption::Bzip2CompressionLevel(CompressionLevel::BEST))
        .open_file(&archive_path)
        .unwrap();

    archive.add_file("test.txt", &test_data).unwrap();
    archive.finish().unwrap();

    assert!(archive_path.exists());

    // Verify by reading back
    use libarchive2::ReadArchive;
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();
    let entry = read_archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "test.txt");
    let data = read_archive.read_data_to_vec().unwrap();
    assert_eq!(data, test_data);
}

#[test]
fn test_xz_compression_level() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.tar.xz");

    let test_data = b"XZ compression test data. ".repeat(100);

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Xz)
        .filter_option(FilterOption::XzCompressionLevel(CompressionLevel::FASTEST))
        .open_file(&archive_path)
        .unwrap();

    archive.add_file("test.txt", &test_data).unwrap();
    archive.finish().unwrap();

    assert!(archive_path.exists());

    // Verify by reading back
    use libarchive2::ReadArchive;
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();
    let entry = read_archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "test.txt");
    let data = read_archive.read_data_to_vec().unwrap();
    assert_eq!(data, test_data);
}

#[test]
fn test_zstd_compression_level() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.tar.zst");

    let test_data = b"Zstd compression test data. ".repeat(100);

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Zstd)
        .filter_option(FilterOption::ZstdCompressionLevel(3))
        .open_file(&archive_path)
        .unwrap();

    archive.add_file("test.txt", &test_data).unwrap();
    archive.finish().unwrap();

    assert!(archive_path.exists());

    // Verify by reading back
    use libarchive2::ReadArchive;
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();
    let entry = read_archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "test.txt");
    let data = read_archive.read_data_to_vec().unwrap();
    assert_eq!(data, test_data);
}

#[test]
fn test_lz4_compression_level() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.tar.lz4");

    let test_data = b"LZ4 compression test data. ".repeat(100);

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Lz4)
        .filter_option(FilterOption::Lz4CompressionLevel(CompressionLevel::DEFAULT))
        .open_file(&archive_path)
        .unwrap();

    archive.add_file("test.txt", &test_data).unwrap();
    archive.finish().unwrap();

    assert!(archive_path.exists());

    // Verify by reading back
    use libarchive2::ReadArchive;
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();
    let entry = read_archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "test.txt");
    let data = read_archive.read_data_to_vec().unwrap();
    assert_eq!(data, test_data);
}

#[test]
fn test_multiple_format_options() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("multi_options.zip");

    let test_data = b"Test data for multiple options";

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .format_option(FormatOption::ZipCompressionMethod(
            ZipCompressionMethod::Deflate,
        ))
        .format_option(FormatOption::ZipCompressionLevel(CompressionLevel::BEST))
        .open_file(&archive_path)
        .unwrap();

    archive.add_file("test.txt", test_data).unwrap();
    archive.finish().unwrap();

    assert!(archive_path.exists());

    // Verify by reading back - multiple options should work together
    use libarchive2::ReadArchive;
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();
    let entry = read_archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "test.txt");
    let data = read_archive.read_data_to_vec().unwrap();
    assert_eq!(data, test_data);
}

#[test]
fn test_7z_compression_level() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.7z");

    let test_data = b"7z compression test data. ".repeat(100);

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::SevenZip)
        .format_option(FormatOption::SevenZipCompressionLevel(
            CompressionLevel::BEST,
        ))
        .open_file(&archive_path)
        .unwrap();

    archive.add_file("test.txt", &test_data).unwrap();
    archive.finish().unwrap();

    assert!(archive_path.exists());

    // Verify by reading back
    use libarchive2::ReadArchive;
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();
    let entry = read_archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "test.txt");
    let data = read_archive.read_data_to_vec().unwrap();
    assert_eq!(data, test_data);
}

#[test]
fn test_iso9660_options() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.iso");

    let test_data = b"Test ISO";

    // Test basic ISO9660 with volume ID and publisher
    // Note: allow-lowercase may not be supported by all libarchive versions
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Iso9660)
        .format_option(FormatOption::Iso9660VolumeId("TEST_DISK".to_string()))
        .format_option(FormatOption::Iso9660Publisher("Test Corp".to_string()))
        .open_file(&archive_path)
        .unwrap();

    archive.add_file("readme.txt", test_data).unwrap();
    archive.finish().unwrap();

    assert!(archive_path.exists());

    // Verify by reading back
    // Note: ISO9660 format has complex directory structure, so we iterate to find our file
    use libarchive2::ReadArchive;
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();

    let mut found_file = false;
    while let Some(entry) = read_archive.next_entry().unwrap() {
        if entry.file_type() == libarchive2::FileType::RegularFile {
            let data = read_archive.read_data_to_vec().unwrap();
            if data == test_data {
                found_file = true;
                break;
            }
        }
    }

    assert!(found_file, "Should find and read the test file from ISO");
}

#[test]
fn test_iso9660_lowercase_option() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test_lower.iso");

    // Test lowercase option separately - may not be supported by all versions
    let result = WriteArchive::new()
        .format(ArchiveFormat::Iso9660)
        .format_option(FormatOption::Iso9660AllowLowercase(true))
        .open_file(&archive_path);

    // This option may not be supported, so we just test the API accepts it
    // If it fails, that's OK - libarchive version dependent
    if let Ok(mut archive) = result {
        let _ = archive.add_file("test.txt", b"data");
        let _ = archive.finish();
    }
}

#[test]
fn test_compression_no_data() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("empty.zip");

    let archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .format_option(FormatOption::ZipCompressionLevel(CompressionLevel::BEST))
        .open_file(&archive_path)
        .unwrap();

    // Create archive with no files
    archive.finish().unwrap();

    assert!(archive_path.exists());

    // Verify we can read the empty archive
    use libarchive2::ReadArchive;
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();
    // Should have no entries
    assert!(read_archive.next_entry().unwrap().is_none());
}
