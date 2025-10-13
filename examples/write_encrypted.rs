//! Example: Creating encrypted archives
//!
//! This example demonstrates how to create password-protected archives
//! in various formats (ZIP, 7z).

use libarchive2::{ArchiveFormat, WriteArchive};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <output_file> <password>", args[0]);
        eprintln!("\nExample: {} encrypted.zip my_secret_password", args[0]);
        eprintln!("Supported formats: .zip, .7z");
        std::process::exit(1);
    }

    let output_file = &args[1];
    let password = &args[2];

    // Determine format from extension
    let format = if output_file.ends_with(".zip") {
        ArchiveFormat::Zip
    } else if output_file.ends_with(".7z") {
        ArchiveFormat::SevenZip
    } else {
        eprintln!("Unsupported format. Use .zip or .7z extension.");
        std::process::exit(1);
    };

    println!("Creating encrypted archive: {}", output_file);
    println!("Format: {:?}", format);
    println!();

    // Create an encrypted archive
    let mut archive = WriteArchive::new()
        .format(format)
        .passphrase(password)
        .open_file(output_file)?;

    // Add some files with sensitive content
    archive.add_file("secret.txt", b"This is secret information!")?;
    archive.add_file("passwords.txt", b"admin:super_secret_password")?;
    archive.add_directory("confidential")?;
    archive.add_file(
        "confidential/data.txt",
        b"This file is in a confidential directory.",
    )?;

    archive.finish()?;

    println!("Encrypted archive created successfully!");
    println!();
    println!("To extract, use:");
    println!(
        "  cargo run --example read_encrypted_archive {} {}",
        output_file, password
    );

    Ok(())
}
