//! Example: Create a tar.gz archive

use libarchive2::{ArchiveFormat, CompressionFormat, WriteArchive};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating archive...");

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Gzip)
        .open_file("example.tar.gz")?;

    // Add a text file
    archive.add_file("hello.txt", b"Hello, world!")?;

    // Add a directory
    archive.add_directory("mydir")?;

    // Add a file in the directory
    archive.add_file("mydir/file.txt", b"File in directory")?;

    archive.finish()?;

    println!("Archive created successfully: example.tar.gz");
    Ok(())
}
