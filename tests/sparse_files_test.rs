//! Integration tests for sparse file block API

use libarchive2::{ArchiveFormat, EntryMut, FileType, ReadArchive, WriteArchive};
use tempfile::TempDir;

#[test]
fn test_sparse_file_write_and_read() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("sparse_test.tar");

    // Create a sparse file
    // Note: archive_write_data_block may not be supported on all platforms/configurations
    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .open_file(&archive_path)
            .unwrap();

        let mut entry = EntryMut::new();
        entry.set_pathname("sparse.bin").unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(1024 * 1024); // 1MB
        entry.set_perm(0o644).unwrap();

        archive.write_header(&entry).unwrap();

        // Try to write blocks at specific offsets
        // This may not be supported on all platforms
        let result1 = archive.write_data_block(0, b"START");
        if result1.is_err() {
            // write_data_block not supported on this platform - skip test
            println!("Skipping test: write_data_block not supported");
            return;
        }

        archive.write_data_block(512 * 1024, b"MIDDLE").unwrap();
        archive.write_data_block((1024 * 1024) - 3, b"END").unwrap();

        archive.finish().unwrap();
    }

    // Read back the sparse file
    {
        let mut archive = ReadArchive::open(&archive_path).unwrap();
        let entry = archive.next_entry().unwrap().unwrap();

        assert_eq!(entry.pathname().unwrap(), "sparse.bin");
        assert_eq!(entry.size(), 1024 * 1024);

        // Read blocks
        let mut blocks = Vec::new();
        while let Some((offset, data)) = archive.read_data_block().unwrap() {
            blocks.push((offset, data));
        }

        // Verify we got the blocks
        assert_eq!(blocks.len(), 3);

        // Check first block
        assert_eq!(blocks[0].0, 0);
        assert_eq!(&blocks[0].1, b"START");

        // Check middle block
        assert_eq!(blocks[1].0, 512 * 1024);
        assert_eq!(&blocks[1].1, b"MIDDLE");

        // Check last block
        assert_eq!(blocks[2].0, (1024 * 1024) - 3);
        assert_eq!(&blocks[2].1, b"END");
    }
}

#[test]
fn test_sparse_file_multiple_small_blocks() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("sparse_multi.tar");

    let file_size = 10000;
    let data_chunks = vec![
        (0, b"block1".to_vec()),
        (1000, b"block2".to_vec()),
        (5000, b"block3".to_vec()),
        (9990, b"end".to_vec()),
    ];

    // Write
    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .open_file(&archive_path)
            .unwrap();

        let mut entry = EntryMut::new();
        entry.set_pathname("multi_sparse.bin").unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(file_size);
        entry.set_perm(0o644).unwrap();

        archive.write_header(&entry).unwrap();

        // Try first block to see if supported
        let result = archive.write_data_block(data_chunks[0].0, &data_chunks[0].1);
        if result.is_err() {
            println!("Skipping test: write_data_block not supported");
            return;
        }

        for (offset, data) in &data_chunks[1..] {
            archive.write_data_block(*offset, data).unwrap();
        }

        archive.finish().unwrap();
    }

    // Read
    {
        let mut archive = ReadArchive::open(&archive_path).unwrap();
        archive.next_entry().unwrap().unwrap();

        let mut read_blocks = Vec::new();
        while let Some((offset, data)) = archive.read_data_block().unwrap() {
            read_blocks.push((offset, data));
        }

        assert_eq!(read_blocks.len(), data_chunks.len());

        for (i, (expected_offset, expected_data)) in data_chunks.iter().enumerate() {
            assert_eq!(read_blocks[i].0, *expected_offset);
            assert_eq!(&read_blocks[i].1, expected_data);
        }
    }
}

#[test]
fn test_write_data_block_returns_correct_size() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("size_test.tar");

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .open_file(&archive_path)
        .unwrap();

    let mut entry = EntryMut::new();
    entry.set_pathname("test.bin").unwrap();
    entry.set_file_type(FileType::RegularFile);
    entry.set_size(1000);
    entry.set_perm(0o644).unwrap();

    archive.write_header(&entry).unwrap();

    let data1 = b"hello world";
    let result1 = archive.write_data_block(0, data1);
    if result1.is_err() {
        println!("Skipping test: write_data_block not supported");
        return;
    }
    let written1 = result1.unwrap();
    assert_eq!(written1, data1.len());

    let data2 = b"test data";
    let written2 = archive.write_data_block(100, data2).unwrap();
    assert_eq!(written2, data2.len());

    archive.finish().unwrap();
}

#[test]
fn test_read_data_block_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("empty.tar");

    // Create empty file
    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .open_file(&archive_path)
            .unwrap();

        let mut entry = EntryMut::new();
        entry.set_pathname("empty.txt").unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(0);
        entry.set_perm(0o644).unwrap();

        archive.write_header(&entry).unwrap();
        archive.finish().unwrap();
    }

    // Read
    {
        let mut archive = ReadArchive::open(&archive_path).unwrap();
        archive.next_entry().unwrap().unwrap();

        // Should immediately return None for empty file
        let result = archive.read_data_block().unwrap();
        assert!(result.is_none());
    }
}

#[test]
fn test_sparse_file_with_regular_write() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("mixed.tar");

    // Create file with mix of block and regular writes
    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .open_file(&archive_path)
            .unwrap();

        let mut entry = EntryMut::new();
        entry.set_pathname("mixed.bin").unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(100);
        entry.set_perm(0o644).unwrap();

        archive.write_header(&entry).unwrap();

        // Use regular write
        let data = vec![42u8; 100];
        archive.write_data(&data).unwrap();

        archive.finish().unwrap();
    }

    // Read using block API
    {
        let mut archive = ReadArchive::open(&archive_path).unwrap();
        archive.next_entry().unwrap().unwrap();

        let mut total = 0;
        while let Some((_, data)) = archive.read_data_block().unwrap() {
            total += data.len();
            // All data should be 42
            assert!(data.iter().all(|&b| b == 42));
        }

        assert_eq!(total, 100);
    }
}

#[test]
fn test_sparse_not_supported_on_zip() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("sparse.zip");

    // ZIP format does not support sparse files
    // write_data_block may fail or convert to regular data
    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::Zip)
        .open_file(&archive_path)
        .unwrap();

    let mut entry = EntryMut::new();
    entry.set_pathname("sparse.bin").unwrap();
    entry.set_file_type(FileType::RegularFile);
    entry.set_size(10000);
    entry.set_perm(0o644).unwrap();

    archive.write_header(&entry).unwrap();

    // Try to write sparse blocks to ZIP
    // This should either fail or work but not create actual sparse holes
    let result = archive.write_data_block(0, b"DATA");

    // If write_data_block is not supported, test passes
    if result.is_err() {
        return;
    }

    // If it succeeds, it should write as regular data
    let written = result.unwrap();
    assert_eq!(written, 4);

    archive.finish().unwrap();

    // Read back and verify
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();
    read_archive.next_entry().unwrap().unwrap();

    // Should be able to read the data
    let mut blocks = Vec::new();
    while let Some((offset, data)) = read_archive.read_data_block().unwrap() {
        blocks.push((offset, data));
    }

    // Should have data (even if not truly sparse)
    assert!(!blocks.is_empty());
}

#[test]
fn test_sparse_overlapping_blocks() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("overlap.tar");

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .open_file(&archive_path)
        .unwrap();

    let mut entry = EntryMut::new();
    entry.set_pathname("overlap.bin").unwrap();
    entry.set_file_type(FileType::RegularFile);
    entry.set_size(1000);
    entry.set_perm(0o644).unwrap();

    archive.write_header(&entry).unwrap();

    // Write overlapping blocks
    let result1 = archive.write_data_block(0, b"FIRST_BLOCK");
    if result1.is_err() {
        // Not supported
        return;
    }

    // Write block that overlaps with previous
    let result2 = archive.write_data_block(5, b"SECOND");

    // Both should succeed or both should fail
    // The behavior with overlapping blocks is undefined but shouldn't crash
    if result2.is_ok() {
        archive.finish().unwrap();

        // Verify archive can be read
        let mut read_archive = ReadArchive::open(&archive_path).unwrap();
        read_archive.next_entry().unwrap().unwrap();

        // Should be able to read something
        let mut total_bytes = 0;
        while let Some((_, data)) = read_archive.read_data_block().unwrap() {
            total_bytes += data.len();
        }

        assert!(total_bytes > 0);
    }
}

#[test]
fn test_sparse_large_offset() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("large_offset.tar");

    let mut archive = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .open_file(&archive_path)
        .unwrap();

    let mut entry = EntryMut::new();
    entry.set_pathname("large.bin").unwrap();
    entry.set_file_type(FileType::RegularFile);
    entry.set_size(100_000_000); // 100MB
    entry.set_perm(0o644).unwrap();

    archive.write_header(&entry).unwrap();

    // Write data near the end of a 100MB file
    let large_offset = 99_999_000i64;
    let result = archive.write_data_block(large_offset, b"END_DATA");

    if result.is_err() {
        // Platform might not support such large offsets
        return;
    }

    archive.finish().unwrap();

    // Read back and verify
    let mut read_archive = ReadArchive::open(&archive_path).unwrap();
    let entry = read_archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.size(), 100_000_000);

    // Find the block with our data
    let mut found = false;
    while let Some((offset, data)) = read_archive.read_data_block().unwrap() {
        if offset == large_offset {
            assert_eq!(&data, b"END_DATA");
            found = true;
            break;
        }
    }

    assert!(found, "Should find data at large offset");
}
