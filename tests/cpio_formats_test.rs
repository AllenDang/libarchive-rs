use libarchive2::{ArchiveFormat, CompressionFormat, ReadArchive, WriteArchive};

fn roundtrip_cpio_format(format: ArchiveFormat) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.cpio");

    {
        let mut archive = WriteArchive::new()
            .format(format)
            .compression(CompressionFormat::None)
            .open_file(&path)
            .unwrap();

        archive.add_file("hello.txt", b"Hello, CPIO!").unwrap();
        archive
            .add_file("subdir/world.txt", b"World!")
            .unwrap();
        archive.add_directory("emptydir").unwrap();
        archive.finish().unwrap();
    }

    // Read back and verify
    let mut archive = ReadArchive::open(&path).unwrap();
    let mut entries = Vec::new();
    while let Some(entry) = archive.next_entry().unwrap() {
        let name = entry.pathname().unwrap_or_default();
        let data = archive.read_data_to_vec().unwrap();
        entries.push((name, data));
    }

    assert_eq!(entries.len(), 3, "format {:?}: expected 3 entries", format);
    assert_eq!(entries[0].0, "hello.txt");
    assert_eq!(entries[0].1, b"Hello, CPIO!");
    assert_eq!(entries[1].0, "subdir/world.txt");
    assert_eq!(entries[1].1, b"World!");
    assert_eq!(entries[2].0, "emptydir");
}

#[test]
fn test_cpio_default() {
    roundtrip_cpio_format(ArchiveFormat::Cpio);
}

#[test]
fn test_cpio_newc() {
    roundtrip_cpio_format(ArchiveFormat::CpioNewc);
}

#[test]
fn test_cpio_odc() {
    roundtrip_cpio_format(ArchiveFormat::CpioOdc);
}

#[test]
fn test_cpio_bin() {
    roundtrip_cpio_format(ArchiveFormat::CpioBin);
}

#[test]
fn test_cpio_extensions_all_same() {
    assert_eq!(ArchiveFormat::Cpio.extension(), "cpio");
    assert_eq!(ArchiveFormat::CpioNewc.extension(), "cpio");
    assert_eq!(ArchiveFormat::CpioOdc.extension(), "cpio");
    assert_eq!(ArchiveFormat::CpioBin.extension(), "cpio");
}
