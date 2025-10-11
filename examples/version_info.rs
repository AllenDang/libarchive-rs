//! Example: Display libarchive version information

fn main() {
    println!("libarchive version: {}", libarchive2::version());
    println!("Version number: {}", libarchive2::version_number());
    println!("\nDetails:\n{}", libarchive2::version_details());
}
