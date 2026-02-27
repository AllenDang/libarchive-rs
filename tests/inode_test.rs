use libarchive2::{
    ArchiveFormat, CompressionFormat, EntryMut, FileType, ReadArchive, WriteArchive,
};

#[test]
fn test_entries_get_unique_inodes() {
    let e1 = EntryMut::new();
    let e2 = EntryMut::new();
    let e3 = EntryMut::new();

    let i1 = e1.as_entry().ino();
    let i2 = e2.as_entry().ino();
    let i3 = e3.as_entry().ino();

    assert_ne!(i1, i2);
    assert_ne!(i2, i3);
    assert_ne!(i1, i3);
    // All should be non-zero
    assert_ne!(i1, 0);
    assert_ne!(i2, 0);
    assert_ne!(i3, 0);
}

#[test]
fn test_cpio_entries_not_treated_as_hardlinks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inodes.cpio");

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::Cpio)
            .compression(CompressionFormat::None)
            .open_file(&path)
            .unwrap();

        archive.add_file("file1.txt", b"content1").unwrap();
        archive.add_file("file2.txt", b"content2").unwrap();
        archive.add_file("file3.txt", b"content3").unwrap();
        archive.finish().unwrap();
    }

    // Read back and verify each entry has unique inode and independent data
    let mut archive = ReadArchive::open(&path).unwrap();
    let mut entries = Vec::new();

    while let Some(entry) = archive.next_entry().unwrap() {
        let name = entry.pathname().unwrap_or_default();
        let ino = entry.ino();
        let hardlink = entry.hardlink();
        let data = archive.read_data_to_vec().unwrap();
        entries.push((name, ino, hardlink, data));
    }

    assert_eq!(entries.len(), 3);

    // All inodes should be unique
    assert_ne!(entries[0].1, entries[1].1);
    assert_ne!(entries[1].1, entries[2].1);
    assert_ne!(entries[0].1, entries[2].1);

    // No entry should be a hardlink
    assert!(entries[0].2.is_none(), "file1 should not be a hardlink");
    assert!(entries[1].2.is_none(), "file2 should not be a hardlink");
    assert!(entries[2].2.is_none(), "file3 should not be a hardlink");

    // Each file should have its own content
    assert_eq!(entries[0].3, b"content1");
    assert_eq!(entries[1].3, b"content2");
    assert_eq!(entries[2].3, b"content3");
}

#[test]
fn test_set_ino_overrides_auto_inode() {
    let mut entry = EntryMut::new();
    let auto_ino = entry.as_entry().ino();
    assert_ne!(auto_ino, 0);

    // Manual override should work
    entry.set_ino(42);
    assert_eq!(entry.as_entry().ino(), 42);
}

#[test]
fn test_cpio_nlink_with_unique_inodes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nlink.cpio");

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::Cpio)
            .compression(CompressionFormat::None)
            .open_file(&path)
            .unwrap();

        let mut entry = EntryMut::new();
        entry.set_pathname("file.txt").unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(5);
        entry.set_perm(0o644).unwrap();
        entry.set_nlink(1);
        archive.write_header(&entry).unwrap();
        archive.write_data(b"hello").unwrap();

        let mut entry2 = EntryMut::new();
        entry2.set_pathname("other.txt").unwrap();
        entry2.set_file_type(FileType::RegularFile);
        entry2.set_size(5);
        entry2.set_perm(0o644).unwrap();
        entry2.set_nlink(1);
        archive.write_header(&entry2).unwrap();
        archive.write_data(b"world").unwrap();

        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();

    let entry = archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "file.txt");
    assert!(entry.hardlink().is_none());
    let data = archive.read_data_to_vec().unwrap();
    assert_eq!(data, b"hello");

    let entry = archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "other.txt");
    assert!(entry.hardlink().is_none());
    let data = archive.read_data_to_vec().unwrap();
    assert_eq!(data, b"world");
}
