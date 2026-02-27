use std::time::{Duration, SystemTime};

use libarchive2::{
    ArchiveFormat, CompressionFormat, EntryMut, FileType, ReadArchive, WriteArchive,
};

fn epoch_plus(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
}

#[test]
fn test_default_mtime() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mtime.tar");
    let fixed_time = epoch_plus(1_000_000);

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .default_mtime(fixed_time)
            .open_file(&path)
            .unwrap();

        archive.add_file("a.txt", b"aaa").unwrap();
        archive.add_file("b.txt", b"bbb").unwrap();
        archive.add_directory("dir").unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    while let Some(entry) = archive.next_entry().unwrap() {
        let mtime = entry.mtime().unwrap();
        assert_eq!(
            mtime, fixed_time,
            "Entry {} should have overridden mtime",
            entry.pathname().unwrap_or_default()
        );
    }
}

#[test]
fn test_default_uid_gid() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("uidgid.tar");

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .default_uid(1000)
            .default_gid(2000)
            .open_file(&path)
            .unwrap();

        archive.add_file("file.txt", b"content").unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.uid(), Some(1000));
    assert_eq!(entry.gid(), Some(2000));
}

#[test]
fn test_default_uname_gname() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("names.tar");

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .default_uname("buildbot")
            .default_gname("ci")
            .open_file(&path)
            .unwrap();

        archive.add_file("file.txt", b"content").unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.uname(), Some("buildbot".to_string()));
    assert_eq!(entry.gname(), Some("ci".to_string()));
}

#[test]
fn test_overrides_apply_to_write_header() {
    // Verify overrides apply when using write_header() directly (not just add_file)
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("header.tar");
    let fixed_time = epoch_plus(500_000);

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .default_mtime(fixed_time)
            .default_uid(42)
            .default_gid(99)
            .open_file(&path)
            .unwrap();

        let mut entry = EntryMut::new();
        entry.set_pathname("manual.txt").unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(5);
        entry.set_perm(0o644).unwrap();
        // Intentionally set a different mtime on the entry
        entry.set_mtime(epoch_plus(999_999));
        entry.set_uid(1);
        entry.set_gid(1);

        archive.write_header(&entry).unwrap();
        archive.write_data(b"hello").unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    // The overrides should win over the entry's own values
    assert_eq!(entry.mtime().unwrap(), fixed_time);
    assert_eq!(entry.uid(), Some(42));
    assert_eq!(entry.gid(), Some(99));
}

#[test]
fn test_no_overrides_preserves_entry_values() {
    // Without overrides, entry values should pass through unchanged
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no_override.tar");
    let entry_time = epoch_plus(123_456);

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .open_file(&path)
            .unwrap();

        let mut entry = EntryMut::new();
        entry.set_pathname("file.txt").unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(3);
        entry.set_perm(0o644).unwrap();
        entry.set_mtime(entry_time);
        entry.set_uid(500);
        entry.set_gid(600);

        archive.write_header(&entry).unwrap();
        archive.write_data(b"abc").unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.mtime().unwrap(), entry_time);
    assert_eq!(entry.uid(), Some(500));
    assert_eq!(entry.gid(), Some(600));
}

#[test]
fn test_all_overrides_combined() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("all.tar");
    let fixed_time = epoch_plus(0); // epoch

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .default_mtime(fixed_time)
            .default_uid(0)
            .default_gid(0)
            .default_uname("root")
            .default_gname("wheel")
            .open_file(&path)
            .unwrap();

        archive.add_file("bin/tool", b"#!/bin/sh").unwrap();
        archive.add_directory("etc").unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    while let Some(entry) = archive.next_entry().unwrap() {
        let name = entry.pathname().unwrap_or_default();
        assert_eq!(entry.mtime().unwrap(), fixed_time, "{name}: mtime");
        assert_eq!(entry.uid(), Some(0), "{name}: uid");
        assert_eq!(entry.gid(), Some(0), "{name}: gid");
        assert_eq!(entry.uname(), Some("root".to_string()), "{name}: uname");
        assert_eq!(entry.gname(), Some("wheel".to_string()), "{name}: gname");
    }
}
