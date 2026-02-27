use libarchive2::{
    ArchiveFormat, CompressionFormat, EntryMut, FileType, ReadArchive, WriteArchive,
};

#[test]
fn test_cpio_strip_trailing_slash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stripped.cpio");

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::Cpio)
            .compression(CompressionFormat::None)
            .strip_directory_trailing_slash(true)
            .open_file(&path)
            .unwrap();

        archive.add_directory("usr/local/share/myapp").unwrap();
        archive.add_file("usr/local/bin/hello", b"hi").unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    let name = entry.pathname().unwrap();
    // CPIO: trailing slash should be stripped
    assert_eq!(name, "usr/local/share/myapp");
    assert!(!name.ends_with('/'));

    let entry = archive.next_entry().unwrap().unwrap();
    let name = entry.pathname().unwrap();
    // Files should be unaffected
    assert_eq!(name, "usr/local/bin/hello");
}

#[test]
fn test_cpio_without_strip_has_no_slash_by_default() {
    // CPIO doesn't add trailing slashes even without the option
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("default.cpio");

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::Cpio)
            .compression(CompressionFormat::None)
            .open_file(&path)
            .unwrap();

        archive.add_directory("mydir").unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    let name = entry.pathname().unwrap();
    assert_eq!(name, "mydir");
}

#[test]
fn test_tar_pax_always_has_trailing_slash_on_directories() {
    // Tar formats always add trailing slash per POSIX spec, even with strip option
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dirs.tar");

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .strip_directory_trailing_slash(true)
            .open_file(&path)
            .unwrap();

        archive.add_directory("mydir").unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    let name = entry.pathname().unwrap();
    // Tar writer in libarchive always re-adds the trailing slash
    assert!(
        name.ends_with('/'),
        "tar dirs always get trailing slash: {name}"
    );
}

#[test]
fn test_strip_slash_with_write_header() {
    // Verify it works with manual EntryMut + write_header too
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manual.cpio");

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::Cpio)
            .compression(CompressionFormat::None)
            .strip_directory_trailing_slash(true)
            .open_file(&path)
            .unwrap();

        let mut entry = EntryMut::new();
        // Deliberately set a pathname with trailing slash
        entry.set_pathname("some/dir/").unwrap();
        entry.set_file_type(FileType::Directory);
        entry.set_size(0);
        entry.set_perm(0o755).unwrap();

        archive.write_header(&entry).unwrap();
        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    let name = entry.pathname().unwrap();
    assert_eq!(name, "some/dir");
}

#[test]
fn test_strip_slash_only_affects_directories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mixed.cpio");

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::Cpio)
            .compression(CompressionFormat::None)
            .strip_directory_trailing_slash(true)
            .open_file(&path)
            .unwrap();

        // File with a trailing slash in name (unusual, but shouldn't be stripped since it's not a dir)
        let mut entry = EntryMut::new();
        entry.set_pathname("file_with_slash/").unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(0);
        entry.set_perm(0o644).unwrap();
        archive.write_header(&entry).unwrap();

        archive.finish().unwrap();
    }

    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    let name = entry.pathname().unwrap();
    // Not a directory, so slash should be preserved
    assert_eq!(name, "file_with_slash/");
}
