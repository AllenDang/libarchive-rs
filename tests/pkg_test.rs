use libarchive2::{PkgReader, PkgWriter};

#[test]
fn test_pkg_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_path = dir.path().join("test.pkg");

    // Write a .pkg file
    let mut writer = PkgWriter::new();
    writer
        .add_file("usr/local/bin/hello", b"#!/bin/sh\necho hello\n")
        .unwrap();
    writer
        .add_directory("usr/local/share/myapp")
        .unwrap();
    writer
        .add_file_with_perm("usr/local/bin/tool", b"binary content", 0o755)
        .unwrap();
    writer.write(&pkg_path).unwrap();

    // Read it back
    let mut reader = PkgReader::open(&pkg_path).unwrap();
    let mut files = Vec::new();

    while let Some(entry) = reader.next_entry().unwrap() {
        let name = entry.pathname().unwrap_or_default();
        let data = reader.read_data_to_vec().unwrap();
        files.push((name, data));
    }

    assert_eq!(files.len(), 3);
    assert_eq!(files[0].0, "usr/local/bin/hello");
    assert_eq!(files[0].1, b"#!/bin/sh\necho hello\n");
    assert_eq!(files[1].0, "usr/local/share/myapp");
    assert_eq!(files[2].0, "usr/local/bin/tool");
    assert_eq!(files[2].1, b"binary content");
}

#[test]
fn test_pbzx_decompress_compress_roundtrip() {
    use libarchive2::pbzx;

    let original = b"Test data for pbzx roundtrip";
    let compressed = pbzx::compress(original).unwrap();
    assert!(pbzx::is_pbzx(&compressed));
    let decompressed = pbzx::decompress(&compressed).unwrap();
    assert_eq!(decompressed, original);
}

#[test]
fn test_pkg_empty() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_path = dir.path().join("empty.pkg");

    let writer = PkgWriter::new();
    writer.write(&pkg_path).unwrap();

    let mut reader = PkgReader::open(&pkg_path).unwrap();
    assert!(reader.next_entry().unwrap().is_none());
}

#[test]
fn test_pkg_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_path = dir.path().join("symlink.pkg");

    let mut writer = PkgWriter::new();
    writer.add_file("usr/bin/tool", b"content").unwrap();
    writer
        .add_symlink("usr/bin/tool-link", "tool")
        .unwrap();
    writer.write(&pkg_path).unwrap();

    let mut reader = PkgReader::open(&pkg_path).unwrap();
    let mut entries = Vec::new();
    while let Some(entry) = reader.next_entry().unwrap() {
        entries.push(entry.pathname().unwrap_or_default());
    }
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0], "usr/bin/tool");
    assert_eq!(entries[1], "usr/bin/tool-link");
}
