use libarchive2::ReadArchive;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <archive_file> [destination]", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} archive.tar.gz /tmp/extract", args[0]);
        eprintln!("  {} archive.zip .", args[0]);
        std::process::exit(1);
    }

    let archive_path = &args[1];
    let dest = if args.len() >= 3 { &args[2] } else { "." };

    println!("Extracting archive: {}", archive_path);
    println!("Destination: {}", dest);
    println!();

    // Create destination directory if it doesn't exist
    fs::create_dir_all(dest)?;

    // Open the archive
    let mut archive = ReadArchive::open(archive_path)?;

    println!("Extracting entries:");
    println!("{:-<80}", "");

    let mut count = 0;
    while let Some(entry) = archive.next_entry()? {
        let pathname = entry
            .pathname()
            .unwrap_or_else(|| "<unknown>".to_string())
            .to_string();
        let file_type = entry.file_type();
        let size = entry.size();

        let type_str = match file_type {
            libarchive2::FileType::RegularFile => "File",
            libarchive2::FileType::Directory => "Dir ",
            libarchive2::FileType::SymbolicLink => "Link",
            _ => "Other",
        };

        println!("[{}] {:60} {:>10} bytes", type_str, pathname, size);

        // Manual extraction for demonstration
        let full_path = Path::new(dest).join(&pathname);

        match file_type {
            libarchive2::FileType::Directory => {
                fs::create_dir_all(&full_path)?;
            }
            libarchive2::FileType::RegularFile => {
                // Create parent directories
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Write file data
                let data = archive.read_data_to_vec()?;
                let mut file = fs::File::create(&full_path)?;
                file.write_all(&data)?;
            }
            _ => {
                // Skip other types for now
                archive.skip_data()?;
            }
        }

        count += 1;
    }

    println!("{:-<80}", "");
    println!("Successfully extracted {} entries to: {}", count, dest);

    Ok(())
}
