# libarchive2

Safe Rust bindings for [libarchive](https://github.com/libarchive/libarchive) v3.8.1, providing cross-platform archive reading and writing capabilities.

## Features

- Full libarchive v3.8.1 feature support
- Safe, idiomatic Rust API built on top of FFI bindings
- RAII resource management (automatic cleanup)
- Support for multiple archive formats: TAR, ZIP, 7z, AR, CPIO, ISO9660, XAR, MTREE, WARC, and more
- Support for multiple compression formats: gzip, bzip2, xz, zstd, lz4, compress, and more
- Cross-platform: macOS, Windows, Linux, iOS, and Android

## Architecture

This crate consists of two layers:

1. **libarchive2-sys**: Low-level FFI bindings generated with bindgen
2. **libarchive2**: High-level safe Rust API

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

## Usage

### Reading an Archive

```rust
use libarchive2::{ReadArchive, FileType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .compression(CompressionFormat::Gzip)
        .open_file("output.tar.gz")?;

    // Add a file
    archive.add_file("hello.txt", b"Hello, World!")?;

    // Add a directory
    archive.add_directory("my_directory")?;

    Ok(())
}
```

### Reading from Memory

```rust
use libarchive2::ReadArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive_data: &[u8] = &[/* ... */];
    let mut archive = ReadArchive::open_memory(archive_data)?;

    while let Some(entry) = archive.next_entry()? {
        println!("Entry: {:?}", entry.pathname());
    }

    Ok(())
}
```

## Examples

See the `examples/` directory for more usage examples:

- `version_info.rs`: Display libarchive version information
- `create_archive.rs`: Create a tar.gz archive with files and directories
- `read_archive.rs`: Read and display archive contents

Run examples with:

```bash
cargo run --example version_info
cargo run --example create_archive
cargo run --example read_archive
```

## Supported Formats

### Archive Formats

- TAR (including GNU, PAX, USTAR variants)
- ZIP
- 7-Zip
- AR (Unix archive)
- CPIO
- ISO 9660 (CD-ROM)
- XAR
- MTREE
- RAW
- Shar
- WARC

### Compression Formats

- None (uncompressed)
- Gzip
- Bzip2
- XZ/LZMA
- Zstd
- LZ4
- Compress (LZW)
- UUEncode
- LZIP
- LRZIP
- LZOP
- GRZIP

## License

This project follows the same license as libarchive itself. See the libarchive submodule for license details.

## Contributing

Contributions are welcome! Please ensure that:

1. Code compiles without warnings (`cargo check`, `cargo clippy`)
2. Code follows Rust 2024 edition standards
3. All existing tests pass
4. New features include appropriate tests

## Building from Source

```bash
# Clone with submodules
git clone --recursive https://github.com/AllenDang/libarchive-rs.git
cd libarchive-rs

# Build
cargo build --release

# Run tests
cargo test

# Check for issues
cargo check
cargo clippy
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
