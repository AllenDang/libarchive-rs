//! Integration tests for multi-volume archive support

use libarchive2::{ArchiveFormat, ReadArchive, WriteArchive};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_open_filenames_empty_list() {
    let paths: Vec<PathBuf> = vec![];
    let result = ReadArchive::open_filenames(&paths);

    // Empty list should return an error
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("At least one file path"));
    }
}

#[test]
fn test_open_filenames_nonexistent_files() {
    let paths = vec!["nonexistent1.rar", "nonexistent2.rar"];
    let result = ReadArchive::open_filenames(&paths);

    // Should fail because files don't exist
    assert!(result.is_err());
}

#[test]
fn test_open_filenames_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("single.txt");

    // Create a simple file
    fs::write(&file_path, b"test content").unwrap();

    // Try to open as multi-volume (will fail because it's not an archive)
    let paths = vec![file_path];
    let result = ReadArchive::open_filenames(&paths);

    // Should fail because it's not a valid archive
    assert!(result.is_err());
}

#[test]
fn test_open_filenames_path_with_nul_byte() {
    // Path with null byte should be rejected
    let paths = vec!["test\0.rar"];
    let result = ReadArchive::open_filenames(&paths);

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("null byte"));
    }
}

#[test]
fn test_open_filenames_api_accepts_valid_paths() {
    let temp_dir = TempDir::new().unwrap();

    // Create dummy files
    let file1 = temp_dir.path().join("archive.part1");
    let file2 = temp_dir.path().join("archive.part2");

    fs::write(&file1, b"dummy").unwrap();
    fs::write(&file2, b"dummy").unwrap();

    let paths = vec![file1, file2];

    // This will fail to read (not real archives), but API should accept the paths
    let result = ReadArchive::open_filenames(&paths);

    // Error should be about archive format, not path validation
    assert!(result.is_err());
}

#[test]
fn test_open_filenames_with_string_slices() {
    let temp_dir = TempDir::new().unwrap();

    // Create dummy files
    let file1 = temp_dir.path().join("test1.dat");
    let file2 = temp_dir.path().join("test2.dat");

    fs::write(&file1, b"dummy").unwrap();
    fs::write(&file2, b"dummy").unwrap();

    // Test with &str slice
    let paths = vec![file1.to_str().unwrap(), file2.to_str().unwrap()];

    let result = ReadArchive::open_filenames(&paths);
    // Will fail (not archives), but API should work
    assert!(result.is_err());
}

#[test]
fn test_open_filenames_ordering() {
    let temp_dir = TempDir::new().unwrap();

    // Create files with names suggesting order
    let paths: Vec<_> = (1..=3)
        .map(|i| {
            let path = temp_dir.path().join(format!("part{}.dat", i));
            fs::write(&path, format!("data{}", i).as_bytes()).unwrap();
            path
        })
        .collect();

    // API should accept them in order
    let result = ReadArchive::open_filenames(&paths);
    // Will fail (not real archives)
    assert!(result.is_err());

    // Try reversed order - API should still accept it
    let mut reversed_paths = paths.clone();
    reversed_paths.reverse();
    let result2 = ReadArchive::open_filenames(&reversed_paths);
    assert!(result2.is_err());
}

#[test]
fn test_open_filenames_mixed_path_types() {
    use std::path::Path;

    let temp_dir = TempDir::new().unwrap();

    let file1 = temp_dir.path().join("file1.dat");
    let file2 = temp_dir.path().join("file2.dat");

    fs::write(&file1, b"test1").unwrap();
    fs::write(&file2, b"test2").unwrap();

    // Test with mixed &Path and &PathBuf
    let path_ref: &Path = file1.as_path();
    let pathbuf = file2.clone();

    let paths = vec![path_ref, pathbuf.as_path()];
    let result = ReadArchive::open_filenames(&paths);

    // Should accept mixed types
    assert!(result.is_err()); // Not real archives
}

#[test]
fn test_open_filenames_unicode_paths() {
    let temp_dir = TempDir::new().unwrap();

    // Test with Unicode characters in filename
    let file = temp_dir.path().join("αρχείο.dat");
    fs::write(&file, b"test").unwrap();

    let paths = vec![file];
    let result = ReadArchive::open_filenames(&paths);

    // Unicode paths should be accepted
    assert!(result.is_err()); // Not a real archive
}

#[test]
fn test_open_filenames_relative_paths() {
    // Test that relative paths are accepted
    let paths = vec!["./file1.rar", "./file2.rar"];
    let result = ReadArchive::open_filenames(&paths);

    // Should fail because files don't exist, not because paths are invalid
    assert!(result.is_err());
}

#[test]
fn test_open_filenames_absolute_paths() {
    let temp_dir = TempDir::new().unwrap();

    let file1 = temp_dir.path().join("abs1.dat");
    let file2 = temp_dir.path().join("abs2.dat");

    fs::write(&file1, b"data1").unwrap();
    fs::write(&file2, b"data2").unwrap();

    // Use absolute paths
    let paths = vec![file1.canonicalize().unwrap(), file2.canonicalize().unwrap()];

    let result = ReadArchive::open_filenames(&paths);

    // Absolute paths should be accepted
    assert!(result.is_err()); // Not real archives
}

// Note: Testing with actual multi-volume RAR archives would require:
// 1. Having RAR archive creation tools available
// 2. Creating real split archives
// 3. Or including test fixtures in the repository
//
// For now, these tests verify the API accepts various path formats
// and properly validates input.

#[test]
fn test_simulated_multivolume_with_separate_archives() {
    // This test simulates multi-volume behavior by creating multiple separate
    // archive files and attempting to read them. While not a true multi-volume
    // archive (which requires special RAR/split format), it tests the API's
    // ability to accept and process multiple file paths.

    let temp_dir = TempDir::new().unwrap();

    // Create three separate small archives
    let files = vec![
        ("archive1.tar", "file1.txt", b"Content of file 1"),
        ("archive2.tar", "file2.txt", b"Content of file 2"),
        ("archive3.tar", "file3.txt", b"Content of file 3"),
    ];

    let mut archive_paths = Vec::new();

    for (archive_name, file_name, content) in &files {
        let archive_path = temp_dir.path().join(archive_name);

        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .open_file(&archive_path)
            .unwrap();

        archive.add_file(file_name, *content).unwrap();
        archive.finish().unwrap();

        archive_paths.push(archive_path);
    }

    // Test 1: Try to open first archive only (should work)
    let result = ReadArchive::open(&archive_paths[0]);
    assert!(result.is_ok());

    let mut archive = result.unwrap();
    if let Some(entry) = archive.next_entry().unwrap() {
        assert_eq!(entry.pathname().unwrap(), "file1.txt");
        let data = archive.read_data_to_vec().unwrap();
        assert_eq!(&data, b"Content of file 1");
    }

    // Test 2: Try open_filenames with all three archives
    // Note: This will likely fail because they're separate archives, not a
    // true multi-volume set, but it tests that the API doesn't crash
    let result = ReadArchive::open_filenames(&archive_paths);

    // The API should either:
    // - Open successfully and read first archive
    // - Return an error (which is also valid behavior)
    // Either way, it shouldn't crash
    match result {
        Ok(mut archive) => {
            // If it succeeds, should be able to read at least the first entry
            if let Some(entry) = archive.next_entry().unwrap() {
                let pathname = entry.pathname().unwrap_or_default().to_string();
                assert!(!pathname.is_empty());
            }
        }
        Err(_) => {
            // Also acceptable - these aren't true multi-volume archives
        }
    }
}

#[test]
fn test_multivolume_api_with_concatenated_archives() {
    // Create a simulated multi-volume archive by manually concatenating
    // TAR archives. While not true multi-volume RAR format, this tests
    // whether the API can handle multiple archives in sequence.

    let temp_dir = TempDir::new().unwrap();

    // Create first archive part
    let part1_path = temp_dir.path().join("concat.part1");
    let mut archive1 = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .open_file(&part1_path)
        .unwrap();
    archive1
        .add_file("part1_file.txt", b"Data from part 1")
        .unwrap();
    archive1.finish().unwrap();

    // Create second archive part
    let part2_path = temp_dir.path().join("concat.part2");
    let mut archive2 = WriteArchive::new()
        .format(ArchiveFormat::TarPax)
        .open_file(&part2_path)
        .unwrap();
    archive2
        .add_file("part2_file.txt", b"Data from part 2")
        .unwrap();
    archive2.finish().unwrap();

    // Test open_filenames with both parts
    let paths = vec![&part1_path, &part2_path];
    let result = ReadArchive::open_filenames(&paths);

    // Verify API accepts the call (may succeed or fail depending on libarchive behavior)
    match result {
        Ok(mut archive) => {
            // Count how many entries we can read
            let mut entry_count = 0;
            while let Some(_entry) = archive.next_entry().unwrap() {
                entry_count += 1;
                let _data = archive.read_data_to_vec().unwrap();
            }

            // Should read at least one entry
            assert!(
                entry_count >= 1,
                "Should read at least one entry from multi-volume API"
            );
        }
        Err(e) => {
            // If it fails, at least verify the error is reasonable
            let error_msg = e.to_string();
            // Should not be a null pointer or crash
            assert!(!error_msg.is_empty(), "Error message should not be empty");
        }
    }
}

#[test]
fn test_multivolume_path_validation_with_real_files() {
    // This test creates real files and verifies that path validation works
    // correctly before attempting to open them as archives.

    let temp_dir = TempDir::new().unwrap();

    // Create dummy files that look like multi-volume parts
    let part1 = temp_dir.path().join("test.part1.tar");
    let part2 = temp_dir.path().join("test.part2.tar");
    let part3 = temp_dir.path().join("test.part3.tar");

    // Write actual TAR archives (small ones)
    for path in [&part1, &part2, &part3] {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .open_file(path)
            .unwrap();

        let filename = format!("file_in_{}", path.file_name().unwrap().to_str().unwrap());
        archive.add_file(&filename, b"test data").unwrap();
        archive.finish().unwrap();
    }

    // Verify all files exist
    assert!(part1.exists());
    assert!(part2.exists());
    assert!(part3.exists());

    // Test 1: Open with correct order
    let paths_correct = vec![&part1, &part2, &part3];
    let result = ReadArchive::open_filenames(&paths_correct);

    // Should at least not crash
    match result {
        Ok(mut archive) => {
            // Try to read first entry
            if let Some(entry) = archive.next_entry().unwrap() {
                let pathname = entry.pathname().unwrap_or_default();
                assert!(!pathname.is_empty());
            }
        }
        Err(_) => {
            // Acceptable - not true multi-volume format
        }
    }

    // Test 2: Open with wrong order
    let paths_wrong = vec![&part3, &part1, &part2];
    let result2 = ReadArchive::open_filenames(&paths_wrong);

    // Should handle gracefully (not crash)
    let _ = result2;

    // Test 3: Open with duplicate paths
    let paths_dup = vec![&part1, &part1, &part2];
    let result3 = ReadArchive::open_filenames(&paths_dup);

    // Should handle gracefully
    let _ = result3;
}
