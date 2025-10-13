//! Example: Writing an archive using custom callbacks
//!
//! This example demonstrates how to use custom write callbacks to create
//! archives to any destination that implements the Write trait.

use libarchive2::{ArchiveFormat, CallbackWriter, CompressionFormat, WriteArchive};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a buffer to write to
    let buffer = Vec::new();

    // Create a callback writer from any type that implements Write
    let callback = CallbackWriter::new(buffer);

    // Create an archive using the callback
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Gzip)
        .open_callback(callback)?;

    // Add some files
    archive.add_file("hello.txt", b"Hello, World!")?;
    archive.add_file("test.txt", b"This is a test file.")?;
    archive.add_directory("my_directory")?;

    // Finish writing
    archive.finish()?;

    println!("Archive written to memory buffer!");
    println!("Note: In this redesigned version, the buffer is consumed by the callback.");
    println!("For capturing output, consider writing to a file directly or using Cursor.");

    Ok(())
}
