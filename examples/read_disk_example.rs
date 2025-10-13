//! Example: Using ReadDisk API to create archives from disk
//!
//! This example demonstrates how to use the ReadDisk API to read files
//! from disk and create an archive.

use libarchive2::{ArchiveFormat, CompressionFormat, ReadDisk, SymlinkMode, WriteArchive};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <source_directory> <output_archive>", args[0]);
        eprintln!("\nExample: {} /path/to/dir output.tar.gz", args[0]);
        std::process::exit(1);
    }

    let source_dir = &args[1];
    let output_archive = &args[2];

    println!("Creating archive from directory: {}", source_dir);
    println!("Output archive: {}", output_archive);
    println!();

    // Create the write archive
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Gzip)
        .open_file(output_archive)?;

    // Create a ReadDisk instance
    let mut disk = ReadDisk::new()?;
    disk.set_symlink_mode(SymlinkMode::Physical)?;
    disk.set_standard_lookup()?;

    // Open the directory
    disk.open(source_dir)?;

    let mut file_count = 0;

    // Read entries from disk
    while let Some(entry_mut) = disk.next_entry()? {
        let entry = entry_mut.as_entry();
        let pathname = entry.pathname().unwrap_or_else(|| "<unknown>".to_string());
        println!("Adding: {}", pathname);

        // Write the entry to the archive
        archive.write_header(&entry_mut)?;

        // If it's a regular file, copy the data
        if entry.file_type() == libarchive2::FileType::RegularFile {
            // For files with data, we would need to read from disk
            // ReadDisk doesn't provide read_data - you'd typically use std::fs
            // This is a simplified example
            let size = entry.size();
            if size > 0 {
                println!(
                    "  (File has {} bytes - would need to read from filesystem)",
                    size
                );
            }
        }

        file_count += 1;

        // Descend into directories
        if disk.can_descend() {
            disk.descend()?;
        }
    }

    archive.finish()?;

    println!();
    println!("Archive created successfully!");
    println!("Total files added: {}", file_count);

    Ok(())
}
