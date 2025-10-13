//! Example demonstrating runtime format and compression options
//!
//! This example shows how to use strongly-typed format and filter options
//! to control compression levels, format-specific settings, and metadata.

use libarchive2::{
    ArchiveFormat, CompressionFormat, CompressionLevel, FilterOption, FormatOption, WriteArchive,
    ZipCompressionMethod,
};
use std::error::Error;
use std::fs;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Compression Options Example ===\n");

    // Example 1: ZIP with different compression levels
    create_zip_with_compression_levels()?;

    // Example 2: TAR.GZ with custom compression
    create_tar_gz_with_compression()?;

    // Example 3: ISO9660 with metadata
    create_iso_with_metadata()?;

    // Example 4: 7z with best compression
    create_7z_with_best_compression()?;

    println!("\n✓ All compression examples completed successfully!");
    Ok(())
}

/// Create ZIP archives with different compression levels
fn create_zip_with_compression_levels() -> Result<(), Box<dyn Error>> {
    println!("1. Creating ZIP archives with different compression levels...");

    let test_data = b"This is test data that will be compressed. ".repeat(100);

    // No compression
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .format_option(FormatOption::ZipCompressionMethod(
            ZipCompressionMethod::Store,
        ))
        .open_file("example_no_compression.zip")?;

    archive.add_file("test.txt", &test_data)?;
    archive.finish()?;

    let size_no_compression = fs::metadata("example_no_compression.zip")?.len();
    println!("   - No compression: {} bytes", size_no_compression);

    // Fastest compression (level 1)
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .format_option(FormatOption::ZipCompressionMethod(
            ZipCompressionMethod::Deflate,
        ))
        .format_option(FormatOption::ZipCompressionLevel(CompressionLevel::FASTEST))
        .open_file("example_fastest.zip")?;

    archive.add_file("test.txt", &test_data)?;
    archive.finish()?;

    let size_fastest = fs::metadata("example_fastest.zip")?.len();
    println!("   - Fastest (level 1): {} bytes", size_fastest);

    // Default compression (level 6)
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .format_option(FormatOption::ZipCompressionLevel(CompressionLevel::DEFAULT))
        .open_file("example_default.zip")?;

    archive.add_file("test.txt", &test_data)?;
    archive.finish()?;

    let size_default = fs::metadata("example_default.zip")?.len();
    println!("   - Default (level 6): {} bytes", size_default);

    // Best compression (level 9)
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .format_option(FormatOption::ZipCompressionLevel(CompressionLevel::BEST))
        .open_file("example_best.zip")?;

    archive.add_file("test.txt", &test_data)?;
    archive.finish()?;

    let size_best = fs::metadata("example_best.zip")?.len();
    println!("   - Best (level 9): {} bytes", size_best);

    println!(
        "   Compression ratio: {:.1}%",
        (1.0 - size_best as f64 / size_no_compression as f64) * 100.0
    );

    Ok(())
}

/// Create TAR.GZ with custom gzip compression level
fn create_tar_gz_with_compression() -> Result<(), Box<dyn Error>> {
    println!("\n2. Creating TAR.GZ with custom compression...");

    let test_data = b"Repeated data for compression testing. ".repeat(50);

    // TAR.GZ with best compression
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Gzip)
        .filter_option(FilterOption::GzipCompressionLevel(CompressionLevel::BEST))
        .open_file("example_best.tar.gz")?;

    archive.add_file("data.txt", &test_data)?;
    archive.add_directory("test_dir")?;
    archive.add_file("test_dir/nested.txt", b"Nested file content")?;
    archive.finish()?;

    let size = fs::metadata("example_best.tar.gz")?.len();
    println!("   - Created example_best.tar.gz ({} bytes)", size);

    // TAR.GZ with fastest compression
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Gzip)
        .filter_option(FilterOption::GzipCompressionLevel(
            CompressionLevel::FASTEST,
        ))
        .open_file("example_fastest.tar.gz")?;

    archive.add_file("data.txt", &test_data)?;
    archive.finish()?;

    let size_fastest = fs::metadata("example_fastest.tar.gz")?.len();
    println!(
        "   - Created example_fastest.tar.gz ({} bytes)",
        size_fastest
    );
    println!(
        "   Fastest is {:.1}% larger than best",
        ((size_fastest as f64 / size as f64) - 1.0) * 100.0
    );

    Ok(())
}

/// Create ISO9660 disc image with custom metadata
fn create_iso_with_metadata() -> Result<(), Box<dyn Error>> {
    println!("\n3. Creating ISO9660 with custom metadata...");

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Iso9660)
        .format_option(FormatOption::Iso9660VolumeId("MY_DISK_2025".to_string()))
        .format_option(FormatOption::Iso9660Publisher(
            "Example Corporation".to_string(),
        ))
        .format_option(FormatOption::Iso9660AllowLowercase(true))
        .open_file("example_disk.iso")?;

    archive.add_file(
        "readme.txt",
        b"This is a demo ISO image with custom metadata.",
    )?;
    archive.add_directory("documents")?;
    archive.add_file("documents/doc1.txt", b"Document 1 content")?;
    archive.add_file("documents/doc2.txt", b"Document 2 content")?;
    archive.finish()?;

    let size = fs::metadata("example_disk.iso")?.len();
    println!("   - Created example_disk.iso ({} bytes)", size);
    println!("   - Volume ID: MY_DISK_2025");
    println!("   - Publisher: Example Corporation");
    println!("   - Lowercase filenames: enabled");

    Ok(())
}

/// Create 7z archive with best compression
fn create_7z_with_best_compression() -> Result<(), Box<dyn Error>> {
    println!("\n4. Creating 7z archive with best compression...");

    let test_data = b"This is data for 7z compression. ".repeat(100);

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::SevenZip)
        .format_option(FormatOption::SevenZipCompressionLevel(
            CompressionLevel::BEST,
        ))
        .open_file("example_best.7z")?;

    archive.add_file("data.txt", &test_data)?;
    archive.add_file("data2.txt", &test_data)?;
    archive.add_file("data3.txt", &test_data)?;
    archive.finish()?;

    let size = fs::metadata("example_best.7z")?.len();
    println!("   - Created example_best.7z ({} bytes)", size);
    println!(
        "   - Compression ratio: {:.1}%",
        (1.0 - size as f64 / (test_data.len() * 3) as f64) * 100.0
    );

    Ok(())
}
