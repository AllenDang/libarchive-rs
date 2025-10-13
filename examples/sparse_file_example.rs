//! Example demonstrating sparse file operations
//!
//! This example shows how to create and read sparse files using the block API.
//! Sparse files contain large gaps (holes) of zeros, and the block API allows
//! efficient handling by only storing the non-zero data.

use libarchive2::{ArchiveFormat, FileType, ReadArchive, WriteArchive};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Sparse File Example ===\n");

    // Create a sparse file in an archive
    create_sparse_archive()?;

    // Read the sparse file back
    read_sparse_archive()?;

    println!("\n✓ Sparse file operations completed successfully!");
    Ok(())
}

/// Create a TAR archive with a sparse file
fn create_sparse_archive() -> Result<(), Box<dyn Error>> {
    println!("1. Creating sparse archive...");

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .open_file("sparse_example.tar")?;

    // Create a 10MB sparse file with data only at specific offsets
    let mut entry = libarchive2::EntryMut::new();
    entry.set_pathname("sparse_file.bin")?;
    entry.set_file_type(FileType::RegularFile);
    entry.set_size(10 * 1024 * 1024); // 10MB total size
    entry.set_perm(0o644)?;
    entry.set_mtime(std::time::SystemTime::now());

    archive.write_header(&entry)?;

    // Write data at offset 0 (first 13 bytes)
    let data1 = b"Start of file";
    archive.write_data_block(0, data1)?;
    println!("   - Wrote {} bytes at offset 0", data1.len());

    // Skip to 5MB and write more data (creates a 5MB hole)
    let offset2 = 5 * 1024 * 1024;
    let data2 = b"Middle of file";
    archive.write_data_block(offset2, data2)?;
    println!(
        "   - Wrote {} bytes at offset {} (5MB hole created)",
        data2.len(),
        offset2
    );

    // Skip to near the end and write final data
    let offset3 = (10 * 1024 * 1024) - 11;
    let data3 = b"End of file";
    archive.write_data_block(offset3, data3)?;
    println!(
        "   - Wrote {} bytes at offset {} (another ~5MB hole)",
        data3.len(),
        offset3
    );

    archive.finish()?;

    println!("   ✓ Created sparse_example.tar");
    println!(
        "   Total file size: 10MB, Actual data: {} bytes",
        data1.len() + data2.len() + data3.len()
    );

    Ok(())
}

/// Read the sparse file using the block API
fn read_sparse_archive() -> Result<(), Box<dyn Error>> {
    println!("\n2. Reading sparse archive...");

    let mut archive = ReadArchive::open("sparse_example.tar")?;

    if let Some(entry) = archive.next_entry()? {
        let pathname = entry.pathname().unwrap_or_default().to_string();
        let size = entry.size();

        println!("   Entry: {}", pathname);
        println!("   Size: {} bytes", size);

        // Read using block API (sparse-aware)
        let mut block_count = 0;
        let mut total_data = 0;

        while let Some((offset, data)) = archive.read_data_block()? {
            block_count += 1;
            total_data += data.len();

            // Show first few bytes of each block
            let preview = if data.len() > 20 {
                format!("{}...", String::from_utf8_lossy(&data[..20]))
            } else {
                String::from_utf8_lossy(&data).to_string()
            };

            println!(
                "   Block {}: offset={}, size={} bytes, preview='{}'",
                block_count,
                offset,
                data.len(),
                preview
            );
        }

        println!(
            "\n   ✓ Read {} blocks, {} total bytes of actual data",
            block_count, total_data
        );
        println!(
            "   Efficiency: {:.2}% (only non-zero data read)",
            (total_data as f64 / size as f64) * 100.0
        );
    }

    Ok(())
}
