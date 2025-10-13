//! Example: Filtering archive entries using pattern matching
//!
//! This example demonstrates how to use the ArchiveMatch API to filter
//! entries based on patterns, timestamps, and other criteria.

use libarchive2::{ArchiveMatch, ReadArchive};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <archive_file>", args[0]);
        eprintln!("\nThis example demonstrates filtering archive entries.");
        eprintln!("It will show only .txt files from the archive.");
        std::process::exit(1);
    }

    let archive_path = &args[1];

    // Open the archive
    let mut archive = ReadArchive::open(archive_path)?;
    println!("Reading archive: {}", archive_path);
    println!();

    // Create a matcher and configure filters
    let mut matcher = ArchiveMatch::new()?;

    // Include only .txt files
    matcher.include_pattern("*.txt")?;

    // You can also exclude specific patterns
    // matcher.exclude_pattern("*.tmp")?;

    // Or filter by time (e.g., only files from the last 7 days)
    // let one_week_ago = SystemTime::now()
    //     .duration_since(UNIX_EPOCH)?
    //     .as_secs() - (7 * 24 * 60 * 60);
    // matcher.include_time_newer_than(one_week_ago as i64, 0)?;

    println!("Filtering for: *.txt files");
    println!("{}", "=".repeat(60));

    let mut matched_count = 0;
    let mut total_count = 0;

    // Read entries and apply filter
    while let Some(entry) = archive.next_entry()? {
        total_count += 1;
        let pathname = entry.pathname().unwrap_or_else(|| "<unknown>".to_string());

        if matcher.matches(&entry)? {
            matched_count += 1;
            println!("✓ {}", pathname);
            println!("  Type: {:?}", entry.file_type());
            println!("  Size: {} bytes", entry.size());

            if let Some(mtime) = entry.mtime() {
                println!("  Modified: {:?}", mtime);
            }
            println!();
        }
    }

    println!("{}", "=".repeat(60));
    println!("Matched {} of {} entries", matched_count, total_count);

    Ok(())
}
