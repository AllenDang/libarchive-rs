//! pbzx stream decompression for macOS .pkg Payload files
//!
//! pbzx is Apple's chunked XZ compression format used in macOS `.pkg` installer files.
//! The Payload inside a `.pkg` (which is an XAR archive) is typically pbzx-compressed,
//! containing a CPIO archive with the actual package contents.
//!
//! # Format
//!
//! A pbzx stream consists of:
//! - 4-byte magic: `pbzx`
//! - 8-byte big-endian: uncompressed chunk size
//! - Repeating chunks, each with:
//!   - 8-byte big-endian flags (0x01000000 = XZ compressed, otherwise raw)
//!   - 8-byte big-endian compressed length
//!   - Compressed (or raw) data of that length
//!
//! # Examples
//!
//! ```no_run
//! use libarchive2::pbzx;
//!
//! let payload_data = std::fs::read("Payload")?;
//! let decompressed = pbzx::decompress(&payload_data)?;
//! // decompressed is now a CPIO archive that can be read with ReadArchive::open_memory
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::error::{Error, Result};
use std::io::Cursor;

/// Magic bytes identifying a pbzx stream
const PBZX_MAGIC: &[u8; 4] = b"pbzx";

/// Flag indicating a chunk is XZ-compressed
const CHUNK_FLAG_XZ: u64 = 0x0100_0000;

/// Decompress a pbzx stream into its raw content (typically CPIO)
///
/// # Arguments
///
/// * `data` - The raw pbzx stream bytes
///
/// # Returns
///
/// The decompressed data as a `Vec<u8>`
///
/// # Errors
///
/// Returns an error if:
/// - The data is too short or missing the `pbzx` magic header
/// - A chunk header is truncated
/// - XZ decompression of a chunk fails
///
/// # Examples
///
/// ```no_run
/// use libarchive2::pbzx;
///
/// let payload = std::fs::read("Payload")?;
/// let cpio_data = pbzx::decompress(&payload)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 12 {
        return Err(Error::InvalidArgument(
            "Data too short to be a pbzx stream".to_string(),
        ));
    }

    // Verify magic
    if &data[..4] != PBZX_MAGIC {
        return Err(Error::InvalidArgument(
            "Not a pbzx stream (missing magic header)".to_string(),
        ));
    }

    // Read default chunk size (8 bytes big-endian)
    let _default_chunk_size = u64::from_be_bytes(
        data[4..12]
            .try_into()
            .map_err(|_| Error::InvalidArgument("Invalid pbzx header".to_string()))?,
    );

    let mut output = Vec::new();
    let mut offset = 12;

    while offset < data.len() {
        // Read chunk flags (8 bytes big-endian)
        if offset + 16 > data.len() {
            break;
        }

        let flags = u64::from_be_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| Error::InvalidArgument("Invalid chunk header".to_string()))?,
        );
        offset += 8;

        let compressed_size = u64::from_be_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| Error::InvalidArgument("Invalid chunk header".to_string()))?,
        ) as usize;
        offset += 8;

        if offset + compressed_size > data.len() {
            return Err(Error::InvalidArgument(format!(
                "Chunk data truncated: need {} bytes at offset {}, but only {} bytes remain",
                compressed_size,
                offset,
                data.len() - offset
            )));
        }

        let chunk_data = &data[offset..offset + compressed_size];
        offset += compressed_size;

        if flags == CHUNK_FLAG_XZ {
            // XZ-compressed chunk
            let mut decompressed = Vec::new();
            lzma_rs::xz_decompress(&mut Cursor::new(chunk_data), &mut decompressed).map_err(
                |e| Error::InvalidArgument(format!("Failed to decompress XZ chunk: {}", e)),
            )?;
            output.extend_from_slice(&decompressed);
        } else {
            // Raw (uncompressed) chunk
            output.extend_from_slice(chunk_data);
        }
    }

    Ok(output)
}

/// Default chunk size used by Apple's pbzx (1 MiB)
const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Compress raw data (typically CPIO) into a pbzx stream
///
/// The data is split into chunks and each chunk is XZ-compressed.
/// The default chunk size of 1 MiB matches Apple's tooling.
///
/// # Arguments
///
/// * `data` - The raw data to compress
///
/// # Returns
///
/// The pbzx-framed stream as a `Vec<u8>`
///
/// # Examples
///
/// ```
/// use libarchive2::pbzx;
///
/// let original = b"Hello, world!";
/// let compressed = pbzx::compress(original).unwrap();
/// let decompressed = pbzx::decompress(&compressed).unwrap();
/// assert_eq!(decompressed, original);
/// ```
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    compress_with_chunk_size(data, DEFAULT_CHUNK_SIZE)
}

/// Compress raw data into a pbzx stream with a custom chunk size
///
/// # Arguments
///
/// * `data` - The raw data to compress
/// * `chunk_size` - Size of each chunk before compression (must be > 0)
///
/// # Examples
///
/// ```
/// use libarchive2::pbzx;
///
/// let original = vec![0u8; 4096];
/// let compressed = pbzx::compress_with_chunk_size(&original, 1024).unwrap();
/// let decompressed = pbzx::decompress(&compressed).unwrap();
/// assert_eq!(decompressed, original);
/// ```
pub fn compress_with_chunk_size(data: &[u8], chunk_size: usize) -> Result<Vec<u8>> {
    if chunk_size == 0 {
        return Err(Error::InvalidArgument(
            "Chunk size must be greater than 0".to_string(),
        ));
    }

    let mut output = Vec::new();

    // Write header
    output.extend_from_slice(PBZX_MAGIC);
    output.extend_from_slice(&(chunk_size as u64).to_be_bytes());

    // Compress each chunk
    for chunk in data.chunks(chunk_size) {
        let mut compressed = Vec::new();
        lzma_rs::xz_compress(&mut Cursor::new(chunk), &mut compressed)
            .map_err(|e| Error::InvalidArgument(format!("Failed to XZ-compress chunk: {}", e)))?;

        // Only use compressed data if it's actually smaller
        if compressed.len() < chunk.len() {
            output.extend_from_slice(&CHUNK_FLAG_XZ.to_be_bytes());
            output.extend_from_slice(&(compressed.len() as u64).to_be_bytes());
            output.extend_from_slice(&compressed);
        } else {
            // Store raw if compression didn't help
            output.extend_from_slice(&0u64.to_be_bytes());
            output.extend_from_slice(&(chunk.len() as u64).to_be_bytes());
            output.extend_from_slice(chunk);
        }
    }

    Ok(output)
}

/// Check whether the given data looks like a pbzx stream
///
/// Returns `true` if the data starts with the `pbzx` magic bytes.
///
/// # Examples
///
/// ```
/// use libarchive2::pbzx;
///
/// assert!(pbzx::is_pbzx(b"pbzx\x00\x00\x00\x00\x00\x01\x00\x00"));
/// assert!(!pbzx::is_pbzx(b"not a pbzx stream"));
/// ```
pub fn is_pbzx(data: &[u8]) -> bool {
    data.len() >= 4 && &data[..4] == PBZX_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pbzx_valid() {
        let data = b"pbzx\x00\x00\x00\x00\x00\x01\x00\x00";
        assert!(is_pbzx(data));
    }

    #[test]
    fn test_is_pbzx_invalid() {
        assert!(!is_pbzx(b"notpbzx"));
        assert!(!is_pbzx(b""));
        assert!(!is_pbzx(b"pbz"));
    }

    #[test]
    fn test_decompress_too_short() {
        let result = decompress(b"pbzx");
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_bad_magic() {
        let result = decompress(b"notpbzx_stream!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_empty_stream() {
        // Valid header with no chunks
        let mut data = Vec::new();
        data.extend_from_slice(b"pbzx");
        data.extend_from_slice(&0x10000u64.to_be_bytes()); // chunk size
        let result = decompress(&data).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_decompress_raw_chunk() {
        let mut data = Vec::new();
        data.extend_from_slice(b"pbzx");
        data.extend_from_slice(&0x10000u64.to_be_bytes()); // default chunk size

        // Raw chunk (flags != CHUNK_FLAG_XZ)
        let payload = b"Hello, world!";
        data.extend_from_slice(&0u64.to_be_bytes()); // flags = 0 (raw)
        data.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        data.extend_from_slice(payload);

        let result = decompress(&data).unwrap();
        assert_eq!(result, b"Hello, world!");
    }

    #[test]
    fn test_decompress_xz_chunk() {
        // Create XZ-compressed data using lzma-rs
        let original = b"The quick brown fox jumps over the lazy dog";
        let mut compressed = Vec::new();
        lzma_rs::xz_compress(&mut Cursor::new(original.as_slice()), &mut compressed).unwrap();

        let mut data = Vec::new();
        data.extend_from_slice(b"pbzx");
        data.extend_from_slice(&0x10000u64.to_be_bytes());

        // XZ chunk
        data.extend_from_slice(&CHUNK_FLAG_XZ.to_be_bytes());
        data.extend_from_slice(&(compressed.len() as u64).to_be_bytes());
        data.extend_from_slice(&compressed);

        let result = decompress(&data).unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn test_decompress_mixed_chunks() {
        let part1 = b"Hello, ";
        let part2 = b"world!";

        // XZ-compress part2
        let mut compressed_part2 = Vec::new();
        lzma_rs::xz_compress(&mut Cursor::new(part2.as_slice()), &mut compressed_part2).unwrap();

        let mut data = Vec::new();
        data.extend_from_slice(b"pbzx");
        data.extend_from_slice(&0x10000u64.to_be_bytes());

        // Raw chunk
        data.extend_from_slice(&0u64.to_be_bytes());
        data.extend_from_slice(&(part1.len() as u64).to_be_bytes());
        data.extend_from_slice(part1);

        // XZ chunk
        data.extend_from_slice(&CHUNK_FLAG_XZ.to_be_bytes());
        data.extend_from_slice(&(compressed_part2.len() as u64).to_be_bytes());
        data.extend_from_slice(&compressed_part2);

        let result = decompress(&data).unwrap();
        assert_eq!(result, b"Hello, world!");
    }

    #[test]
    fn test_compress_roundtrip() {
        let original =
            b"The quick brown fox jumps over the lazy dog. Repeated data helps compression!";
        let compressed = compress(original).unwrap();
        assert!(is_pbzx(&compressed));
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_compress_roundtrip_large() {
        // Test with data larger than one chunk
        let original: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();
        let compressed = compress_with_chunk_size(&original, 1024).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_compress_empty() {
        let compressed = compress(b"").unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_compress_zero_chunk_size() {
        let result = compress_with_chunk_size(b"data", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_truncated_chunk() {
        let mut data = Vec::new();
        data.extend_from_slice(b"pbzx");
        data.extend_from_slice(&0x10000u64.to_be_bytes());

        // Chunk that claims to be 100 bytes but data is shorter
        data.extend_from_slice(&0u64.to_be_bytes());
        data.extend_from_slice(&100u64.to_be_bytes());
        data.extend_from_slice(b"short");

        let result = decompress(&data);
        assert!(result.is_err());
    }
}
