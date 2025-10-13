# libarchive2

Safe Rust bindings for [libarchive](https://github.com/libarchive/libarchive) v3.8.1, providing cross-platform archive reading and writing capabilities.

[![CI](https://github.com/AllenDang/libarchive-rs/workflows/CI/badge.svg)](https://github.com/AllenDang/libarchive-rs/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-BSD-blue.svg)](LICENSE)
[![docs.rs](https://docs.rs/libarchive2/badge.svg)](https://docs.rs/libarchive2)
[![Crates.io](https://img.shields.io/crates/v/libarchive2.svg)](https://crates.io/crates/libarchive2)

## Features

- **Memory Safe**: Comprehensive lifetime tracking prevents use-after-free bugs
- **Type Safe**: Idiomatic Rust API with strong typing and error handling
- **Zero Cost Abstractions**: RAII resource management with no runtime overhead
- **Full Feature Support**: All libarchive v3.8.1 capabilities exposed safely
- **Multiple Formats**: TAR, ZIP, 7z, AR, CPIO, ISO9660, XAR, MTREE, WARC, and more
- **Multiple Compressions**: gzip, bzip2, xz, zstd, lz4, compress, and more
- **Cross-Platform**: macOS, Windows, Linux, iOS, and Android
- **Encryption Support**: Read and write password-protected archives
- **Streaming I/O**: Custom callback support for network streams and custom sources
- **ACL & Extended Attributes**: Full support for advanced file metadata

## Why libarchive2?

This crate provides a **production-ready**, **memory-safe** Rust interface to libarchive with:

- ✅ **Compile-time safety checks** preventing common FFI errors
- ✅ **Comprehensive lifetime management** eliminating use-after-free bugs
- ✅ **Proper error propagation** with idiomatic `Result` types
- ✅ **Thread-safe design** (`Send` but not `Sync` - matching libarchive semantics)
- ✅ **Builder pattern** for ergonomic API construction
- ✅ **Extensive documentation** with safety guarantees explained
- ✅ **Zero warnings** from clippy and rustc

## Architecture

This crate consists of two layers:

1. **libarchive2-sys**: Low-level FFI bindings generated with bindgen
2. **libarchive2**: High-level safe Rust API with lifetime tracking and error handling

## Platform Support

All platforms have been tested with cross-compilation from macOS (ARM64). The build system automatically configures appropriate toolchains and library dependencies for each target platform.

### macOS (x86_64, aarch64)

**Prerequisites:**

```bash
brew install zlib bzip2 xz zstd lz4 libb2 libxml2
```

**Build:**

```bash
cargo build
```

### Linux (x86_64, aarch64)

**Native Build Prerequisites (Debian/Ubuntu):**

```bash
sudo apt-get install build-essential cmake pkg-config \
    zlib1g-dev libbz2-dev liblzma-dev libzstd-dev liblz4-dev
```

**Native Build Prerequisites (Fedora/RHEL):**

```bash
sudo dnf install gcc-c++ cmake pkgconf \
    zlib-devel bzip2-devel xz-devel libzstd-devel lz4-devel
```

**Native Build:**

```bash
cargo build
```

**Cross-Compilation from macOS:**

```bash
# Install Linux cross-compiler toolchain
brew install x86_64-unknown-linux-gnu

# Add target
rustup target add x86_64-unknown-linux-gnu

# Build
cargo build --target x86_64-unknown-linux-gnu
```

### Windows (x86_64)

**Native Build Prerequisites:**

- Visual Studio 2019 or later (with C++ tools) OR MinGW-w64
- CMake 3.15 or later
- vcpkg (recommended for dependencies):
  ```powershell
  vcpkg install zlib bzip2 liblzma zstd lz4
  ```

**Native Build (MSVC):**

```powershell
cargo build --target x86_64-pc-windows-msvc
```

**Native Build (MinGW):**

```bash
cargo build --target x86_64-pc-windows-gnu
```

**Cross-Compilation from macOS/Linux:**

```bash
# Install MinGW toolchain
brew install mingw-w64  # macOS
# or
sudo apt-get install mingw-w64  # Linux

# Add target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --target x86_64-pc-windows-gnu
```

### iOS (aarch64)

**Prerequisites:**

- Xcode with iOS SDK
- Compression libraries (can be built from source or via CocoaPods)

**Build:**

```bash
cargo build --target aarch64-apple-ios
```

Note: You may need to adjust library search paths in your project configuration.

### Android (aarch64, armv7, x86_64, i686)

**Prerequisites:**

- Android NDK r21 or later
- Set `ANDROID_NDK_HOME` environment variable

**Build:**

```bash
# Set NDK path
export ANDROID_NDK_HOME=/path/to/android-ndk

# Build for various Android targets
cargo build --target aarch64-linux-android  # ARM64
cargo build --target armv7-linux-androideabi  # ARMv7
cargo build --target x86_64-linux-android  # x86_64
cargo build --target i686-linux-android  # x86
```

**Features:**

- All compression formats enabled (zlib, bzip2, xz/lzma, zstd, lz4)
- ACL (Access Control Lists) support enabled
- XATTR (Extended Attributes) support enabled

## Platform Support Matrix

| Platform       | Architectures             | Status             | Notes                                               |
| -------------- | ------------------------- | ------------------ | --------------------------------------------------- |
| macOS          | x86_64, ARM64 (M1/M2)     | ✅ Fully Supported | All features enabled                                |
| Windows (GNU)  | x86_64                    | ✅ Supported       | Cross-compilation tested from macOS                 |
| Windows (MSVC) | x86_64                    | ⚠️ Untested        | Should work but not tested                          |
| Linux          | x86_64                    | ✅ Supported       | Cross-compilation tested from macOS                 |
| iOS            | ARM64, x86_64 (simulator) | ✅ Supported       | All features enabled                                |
| Android        | ARM64, ARMv7, x86_64, x86 | ✅ Supported       | All features enabled                                |
| WebAssembly    | wasm32                    | ❌ Not Supported   | libarchive requires POSIX types unavailable in WASM |

### Why WASM is Not Supported

libarchive v3.8.1 is not compatible with WebAssembly because:

- The library requires POSIX types (`pid_t`, `uid_t`, `gid_t`, `mode_t`) that don't exist in WASM
- Depends on system calls and OS-level file system operations not available in WebAssembly
- CMake configuration fails when trying to detect these platform-specific types

If WASM support is critical for your use case, consider using pure-Rust archive libraries like `tar` or `zip` crates instead.

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
libarchive2 = "0.1"
```

## Usage Examples

### Reading an Archive

```rust
use libarchive2::{ReadArchive, FileType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open archive with automatic format/compression detection
    let mut archive = ReadArchive::open("archive.tar.gz")?;

    while let Some(entry) = archive.next_entry()? {
        println!("Entry: {:?}", entry.pathname());
        println!("  Type: {:?}", entry.file_type());
        println!("  Size: {} bytes", entry.size());

        if entry.file_type() == FileType::RegularFile {
            let data = archive.read_data_to_vec()?;
            // Process file data...
        }
    }

    Ok(())
}
```

### Creating an Archive

```rust
use libarchive2::{WriteArchive, ArchiveFormat, CompressionFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Builder pattern for ergonomic API
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Gzip)
        .open_file("output.tar.gz")?;

    // Add a file
    archive.add_file("hello.txt", b"Hello, World!")?;

    // Add a directory
    archive.add_directory("my_directory")?;

    // Archive is automatically closed when dropped
    Ok(())
}
```

### Reading from Memory (Lifetime-Safe)

```rust
use libarchive2::ReadArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive_data: Vec<u8> = std::fs::read("archive.tar.gz")?;

    // Lifetime tracking ensures archive_data cannot be dropped
    // while archive is using it
    let mut archive = ReadArchive::open_memory(&archive_data)?;

    while let Some(entry) = archive.next_entry()? {
        println!("Entry: {:?}", entry.pathname());
    }

    // archive_data can only be dropped after archive is dropped
    Ok(())
}
```

### Writing to Memory (Compile-Time Safety)

```rust
use libarchive2::{WriteArchive, ArchiveFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer
    let mut used = 0;

    // Lifetime parameter ensures buffer lives long enough
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .open_memory(&mut buffer, &mut used)?;

    archive.add_file("test.txt", b"Hello!")?;
    archive.finish()?;

    println!("Archive size: {} bytes", used);
    // buffer is now valid to use with archive data
    Ok(())
}
```

### Reading Encrypted Archives

```rust
use libarchive2::{ReadArchive, FileType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Method 1: Use the convenience function
    let mut archive = ReadArchive::open_with_passphrase(
        "encrypted.zip",
        "my_password"
    )?;

    // Method 2: Add multiple passphrases (tries each in order)
    let mut archive = ReadArchive::new()?;
    archive.support_filter_all()?;
    archive.support_format_all()?;
    archive.add_passphrase("password1")?;
    archive.add_passphrase("password2")?;

    // Read entries as normal
    while let Some(entry) = archive.next_entry()? {
        println!("Entry: {:?}", entry.pathname());
        if entry.file_type() == FileType::RegularFile {
            let data = archive.read_data_to_vec()?;
            // Process decrypted data...
        }
    }

    Ok(())
}
```

### Writing Encrypted Archives

```rust
use libarchive2::{WriteArchive, ArchiveFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ZIP and 7z formats support encryption
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .passphrase("my_secure_password")
        .open_file("encrypted.zip")?;

    archive.add_file("secret.txt", b"Confidential data")?;

    Ok(())
}
```

### Custom Callbacks for Streaming

```rust
use libarchive2::{ReadArchive, CallbackReader};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read from any source implementing std::io::Read
    let file = std::fs::File::open("archive.tar.gz")?;
    let callback = CallbackReader::new(file);
    let mut archive = ReadArchive::open_callback(callback)?;

    while let Some(entry) = archive.next_entry()? {
        println!("Entry: {:?}", entry.pathname());
    }

    Ok(())
}
```

### Pattern Matching and Filtering

```rust
use libarchive2::{ReadArchive, ArchiveMatch};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = ReadArchive::open("archive.tar.gz")?;
    let mut matcher = ArchiveMatch::new()?;

    // Include only .txt files
    matcher.include_pattern("*.txt")?;

    // Exclude temporary files
    matcher.exclude_pattern("*.tmp")?;

    while let Some(entry) = archive.next_entry()? {
        if matcher.matches(&entry)? {
            println!("Matched: {}", entry.pathname().unwrap_or_default());
        }
    }

    Ok(())
}
```

### Reading Directly from Disk

```rust
use libarchive2::{ReadDisk, SymlinkMode, ReadDiskFlags};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut disk = ReadDisk::new()?;
    disk.set_symlink_mode(SymlinkMode::Logical)?;
    disk.set_standard_lookup()?;
    disk.open("/path/to/directory")?;

    while let Some(entry) = disk.next_entry()? {
        println!("File: {}", entry.as_entry().pathname().unwrap_or_default());
        if disk.can_descend() {
            disk.descend()?;
        }
    }

    Ok(())
}
```

## Examples

See the `examples/` directory for comprehensive usage examples:

| Example                     | Description                                        |
| --------------------------- | -------------------------------------------------- |
| `version_info.rs`           | Display libarchive version information             |
| `create_archive.rs`         | Create a tar.gz archive with files and directories |
| `read_archive.rs`           | Read and display archive contents                  |
| `read_encrypted_archive.rs` | Read encrypted/password-protected archives         |
| `write_encrypted.rs`        | Create encrypted ZIP archives                      |
| `callback_write.rs`         | Use custom callbacks for streaming                 |
| `filter_archive.rs`         | Pattern-based filtering of archive entries         |
| `read_disk_example.rs`      | Read files directly from filesystem                |
| `write_disk_example.rs`     | Extract archives to disk                           |
| `extract_archive.rs`        | Full-featured extraction with options              |

Run examples with:

```bash
cargo run --example version_info
cargo run --example create_archive
cargo run --example read_archive
cargo run --example read_encrypted_archive <archive_file> <password>
```

## Supported Formats

### Archive Formats (Read/Write)

| Format      | Read | Write | Notes                     |
| ----------- | ---- | ----- | ------------------------- |
| TAR (POSIX) | ✅   | ✅    | Modern TAR format         |
| TAR (GNU)   | ✅   | ✅    | GNU extensions            |
| TAR (USTAR) | ✅   | ✅    | POSIX.1-1988              |
| ZIP         | ✅   | ✅    | With encryption support   |
| 7-Zip       | ✅   | ✅    | With encryption support   |
| AR          | ✅   | ✅    | Unix archive format       |
| CPIO        | ✅   | ✅    | Traditional Unix format   |
| ISO 9660    | ✅   | ✅    | CD-ROM filesystem         |
| XAR         | ✅   | ✅    | Extensible archive format |
| MTREE       | ✅   | ✅    | BSD manifest format       |
| Shar        | ✅   | ✅    | Shell archive             |
| WARC        | ✅   | ✅    | Web ARChive               |
| RAR         | ✅   | ❌    | Read-only                 |
| RAR5        | ✅   | ❌    | Read-only                 |
| LHA         | ✅   | ❌    | Read-only                 |
| CAB         | ✅   | ❌    | Read-only                 |

### Compression Formats

| Compression | Read | Write | Notes                   |
| ----------- | ---- | ----- | ----------------------- |
| None        | ✅   | ✅    | Uncompressed            |
| Gzip        | ✅   | ✅    | Most common             |
| Bzip2       | ✅   | ✅    | Better compression      |
| XZ/LZMA     | ✅   | ✅    | Best compression        |
| Zstd        | ✅   | ✅    | Fast modern compression |
| LZ4         | ✅   | ✅    | Extremely fast          |
| Compress    | ✅   | ✅    | LZW compression         |
| UUEncode    | ✅   | ✅    | Legacy encoding         |
| LZIP        | ✅   | ❌    | LZMA-based              |
| LRZIP       | ✅   | ✅    | Long-range compression  |
| LZOP        | ✅   | ✅    | LZO-based               |
| GRZIP       | ✅   | ✅    | Grid-friendly           |

## Safety Guarantees

This crate provides strong memory safety guarantees:

- **No use-after-free**: Lifetimes prevent dangling pointers at compile time
- **No data races**: `Send` but not `Sync` enforces single-threaded access
- **No null pointer dereferences**: All null checks before FFI calls
- **No buffer overflows**: All buffer operations bounds-checked
- **Proper error handling**: All FFI errors converted to Rust `Result` types
- **Resource cleanup**: RAII ensures archives are always properly closed

See [COMPREHENSIVE_CODE_REVIEW.md](COMPREHENSIVE_CODE_REVIEW.md) for detailed safety analysis.

## Performance

Zero-cost abstractions mean this crate has **no runtime overhead** compared to using libarchive directly from C:

- No heap allocations beyond what libarchive requires
- Inline function calls eliminate FFI overhead where possible
- Direct memory access with no intermediate copies
- Efficient buffer management with pre-allocated buffers

## License

This project follows the same license as libarchive itself (BSD 2-Clause). See the libarchive submodule for license details.

## Contributing

Contributions are welcome! Please ensure that:

1. ✅ Code compiles without warnings: `cargo check`, `cargo clippy`
2. ✅ Code follows Rust 2024 edition standards
3. ✅ All existing examples build: `cargo build --examples`
4. ✅ New features include documentation with examples
5. ✅ Safety invariants are documented for unsafe code

## Building from Source

```bash
# Clone with submodules
git clone --recursive https://github.com/AllenDang/libarchive-rs.git
cd libarchive-rs

# Build
cargo build --release

# Run all examples
cargo build --examples

# Check for issues
cargo check
cargo clippy --all-targets -- -D warnings

# View documentation
cargo doc --open
```

## Troubleshooting

### macOS: Library Not Found

If you get linker errors on macOS, ensure libraries are installed via Homebrew and try:

```bash
export LIBRARY_PATH=/opt/homebrew/lib:/usr/local/lib:$LIBRARY_PATH
cargo build
```

### Windows: CMake Not Found

Install CMake from https://cmake.org/download/ and add it to your PATH.

### Linux: Missing Development Packages

Ensure all development packages are installed. The exact package names vary by distribution.

### Android: NDK Not Found

Ensure the `ANDROID_NDK_HOME` environment variable is set:

```bash
export ANDROID_NDK_HOME=/path/to/android-ndk
# or
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/25.2.9519653  # Example on macOS
```

### Android: Library Linking Errors

All compression libraries (zlib, bzip2, xz/lzma, zstd, lz4) are enabled and will be linked from the Android NDK. If you encounter linking errors, ensure your NDK version is r21 or later.

### Cross-Compilation: Sysroot Not Found

For cross-compilation (Windows, Linux), ensure the appropriate toolchain is installed via Homebrew:

```bash
# For Windows cross-compilation
brew install mingw-w64

# For Linux cross-compilation
brew install x86_64-unknown-linux-gnu
```
