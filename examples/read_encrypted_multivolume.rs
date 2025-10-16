use libarchive2::{FileType, ReadArchive};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <password> <file1> <file2> [file3...]", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!(
            "  {} my_password encrypted.part1.rar encrypted.part2.rar encrypted.part3.rar",
            args[0]
        );
        std::process::exit(1);
    }

    let password = &args[1];
    let file_paths: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();

    println!(
        "Opening encrypted multi-volume archive with {} parts",
        file_paths.len()
    );
    println!("Using password: {}", "*".repeat(password.len()));
    println!("Parts:");
    for (i, path) in file_paths.iter().enumerate() {
        println!("  {}. {}", i + 1, path);
    }
    println!();

    // Open the encrypted multi-volume archive with a password
    let mut archive = ReadArchive::open_filenames_with_passphrase(&file_paths, password)?;

    println!("Archive contents:");
    println!("{:-<80}", "");

    let mut entry_count = 0;
    let mut total_size = 0i64;

    while let Some(entry) = archive.next_entry()? {
        entry_count += 1;

        let pathname = entry.pathname().unwrap_or_else(|| "<unknown>".to_string());
        let file_type = entry.file_type();
        let size = entry.size();
        total_size += size;

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
    println!("Total size: {} bytes", total_size);

    Ok(())
}
