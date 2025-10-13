use libarchive2::{FileType, ReadArchive};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <archive_file> <password>", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} encrypted.zip my_password", args[0]);
        std::process::exit(1);
    }

    let archive_path = &args[1];
    let password = &args[2];

    println!("Opening encrypted archive: {}", archive_path);
    println!("Using password: {}", "*".repeat(password.len()));
    println!();

    // Open the archive with a password
    let mut archive = ReadArchive::open_with_passphrase(archive_path, password)?;

    println!("Archive contents:");
    println!("{:-<80}", "");

    let mut entry_count = 0;
    while let Some(entry) = archive.next_entry()? {
        entry_count += 1;

        let pathname = entry.pathname().unwrap_or_else(|| "<unknown>".to_string());
        let file_type = entry.file_type();
        let size = entry.size();

        let type_str = match file_type {
            FileType::RegularFile => "File",
            FileType::Directory => "Dir ",
            FileType::SymbolicLink => "Link",
            _ => "Other",
        };

        println!("[{}] {:60} {:>10} bytes", type_str, pathname, size);

        // For demonstration, we can also try to read file contents
        if file_type == FileType::RegularFile && size > 0 && size < 1024 * 1024 {
            // Only read small files (< 1MB) for demo
            let data = archive.read_data_to_vec()?;
            println!("      └─ Read {} bytes successfully", data.len());
        } else {
            // Skip data for directories or large files
            archive.skip_data()?;
        }
    }

    println!("{:-<80}", "");
    println!("Total entries: {}", entry_count);

    Ok(())
}
