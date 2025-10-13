//! Example: Read and extract an archive

use libarchive2::ReadArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "example.tar.gz".to_string());

    println!("Reading archive: {}", path);

    let mut archive = ReadArchive::open(&path)?;

    while let Some(entry) = archive.next_entry()? {
        let pathname = entry.pathname().unwrap_or_else(|| "<unknown>".to_string());
        let size = entry.size();
        let file_type = entry.file_type();

        println!("{:?} {} ({} bytes)", file_type, pathname, size);

        if size > 0 {
            let data = archive.read_data_to_vec()?;
            if data.len() < 100 {
                // Print small files
                if let Ok(text) = std::str::from_utf8(&data) {
                    println!("  Content: {}", text);
                }
            }
        }
    }

    Ok(())
}
