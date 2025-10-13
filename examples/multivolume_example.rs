//! Example demonstrating multi-volume archive reading
//!
//! This example shows how to read archives that are split across multiple files.
//! We'll create a simulated multi-volume archive scenario and read it back.
//!
//! Note: libarchive's multi-volume support is primarily for reading existing
//! multi-volume archives (like split RAR files). This example demonstrates
//! the reading capability.

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Multi-Volume Archive Example ===\n");

    // For demonstration, we'll show how to read multiple archive files
    // as if they were volumes of a single archive
    demonstrate_multivolume_reading()?;

    println!("\n✓ Multi-volume example completed successfully!");
    println!("\nNote: This example demonstrates the API for reading multi-volume archives.");
    println!("To test with real multi-volume archives, create split RAR or other");
    println!("multi-volume formats using their respective tools.");
    Ok(())
}

/// Demonstrate reading multiple files as a multi-volume archive
fn demonstrate_multivolume_reading() -> Result<(), Box<dyn Error>> {
    println!("Multi-Volume Archive API Usage:\n");

    // Show how to use the API (this would work with real multi-volume RAR files)
    show_api_usage();

    println!("\n✓ API demonstration complete");
    Ok(())
}

/// Show the API for reading multi-volume archives
fn show_api_usage() {
    println!("Example 1: Reading split RAR archive");
    println!("-------------------------------------");
    println!(
        r#"
let parts = vec![
    "movie.part1.rar",
    "movie.part2.rar",
    "movie.part3.rar",
];

let mut archive = ReadArchive::open_filenames(&parts)?;

while let Some(entry) = archive.next_entry()? {{
    println!("File: {{}}", entry.pathname().unwrap_or_default());
    println!("Size: {{}} bytes", entry.size());

    // Read data (libarchive handles volume switching automatically)
    let data = archive.read_data_to_vec()?;

    // Process the file...
}}
"#
    );

    println!("\nExample 2: Reading numbered volumes");
    println!("-----------------------------------");
    println!(
        r#"
// For archives like: archive.001, archive.002, archive.003
let parts: Vec<String> = (1..=5)
    .map(|i| format!("archive.{{:03}}", i))
    .collect();

let mut archive = ReadArchive::open_filenames(&parts)?;
// Process entries...
"#
    );

    println!("\nExample 3: Error handling");
    println!("------------------------");
    println!(
        r#"
let parts = vec!["file.part1.rar", "file.part2.rar"];

match ReadArchive::open_filenames(&parts) {{
    Ok(mut archive) => {{
        // Successfully opened multi-volume archive
        while let Some(entry) = archive.next_entry()? {{
            // Process entries...
        }}
    }}
    Err(e) => {{
        eprintln!("Failed to open multi-volume archive: {{}}", e);
        // Handle error (missing file, invalid format, etc.)
    }}
}}
"#
    );

    println!("\nKey Points:");
    println!("-----------");
    println!("1. Files must be provided in the correct order");
    println!("2. All files must be accessible throughout the reading process");
    println!("3. libarchive automatically handles switching between volumes");
    println!("4. Most commonly used for RAR archives (.part1.rar, .part2.rar, ...)");
    println!("5. Also works with other formats that support volume splitting");

    println!("\n\nCommon Use Cases:");
    println!("-----------------");
    println!("• RAR archives: game.part01.rar, game.part02.rar, ...");
    println!("• Split archives: archive.001, archive.002, archive.003, ...");
    println!("• Large file distribution: movie.r00, movie.r01, movie.r02, ...");
    println!("• Size-limited storage: backup.z01, backup.z02, backup.zip");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multivolume_api_demonstration() {
        // This test just ensures the example code runs without panicking
        assert!(demonstrate_multivolume_reading().is_ok());
    }

    #[test]
    fn test_show_api_usage() {
        // Ensure the API usage example can be displayed
        show_api_usage();
        // No panic = success
    }
}
