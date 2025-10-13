//! Example: Using WriteDisk API directly
//!
//! This example demonstrates how to use the low-level WriteDisk API
//! to write archive entries directly to disk with full control.

use libarchive2::{EntryMut, ExtractFlags, FileType, WriteDisk};
use std::time::SystemTime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Writing files to disk using WriteDisk API");
    println!();

    // Create a WriteDisk instance
    let mut disk = WriteDisk::new()?;

    // Set extraction options
    let flags = ExtractFlags::TIME | ExtractFlags::PERM | ExtractFlags::SECURE_SYMLINKS;
    disk.set_options(flags)?;
    disk.set_standard_lookup()?;

    // Create a directory
    let mut dir_entry = EntryMut::new();
    dir_entry.set_pathname("test_output")?;
    dir_entry.set_file_type(FileType::Directory);
    dir_entry.set_perm(0o755)?;
    dir_entry.set_mtime(SystemTime::now());

    disk.write_header(&dir_entry)?;
    disk.finish_entry()?;
    println!("Created directory: test_output/");

    // Create a file
    let mut file_entry = EntryMut::new();
    file_entry.set_pathname("test_output/hello.txt")?;
    file_entry.set_file_type(FileType::RegularFile);
    file_entry.set_size(13);
    file_entry.set_perm(0o644)?;
    file_entry.set_mtime(SystemTime::now());

    disk.write_header(&file_entry)?;
    disk.write_data(b"Hello, World!")?;
    disk.finish_entry()?;
    println!("Created file: test_output/hello.txt");

    // Create another file with more metadata
    let mut file_entry2 = EntryMut::new();
    file_entry2.set_pathname("test_output/metadata_example.txt")?;
    file_entry2.set_file_type(FileType::RegularFile);
    let content = b"This file has extended metadata!";
    file_entry2.set_size(content.len() as i64);
    file_entry2.set_perm(0o644)?;
    file_entry2.set_mtime(SystemTime::now());
    file_entry2.set_atime(SystemTime::now());

    disk.write_header(&file_entry2)?;
    disk.write_data(content)?;
    disk.finish_entry()?;
    println!("Created file: test_output/metadata_example.txt");

    println!();
    println!("All files created successfully!");
    println!("Check the test_output/ directory to see the results.");

    Ok(())
}
