use std::io::{Read, Write};

use libarchive2::{
    ArchiveFormat, CallbackWriter, CompressionFormat, EntryMut, FileType, ReadArchive, WriteArchive,
};

#[test]
fn test_write_archive_io_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("io_write.tar");

    let data = b"Hello from io::Write!";

    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .open_file(&path)
            .unwrap();

        let mut entry = EntryMut::new();
        entry.set_pathname("hello.txt").unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(data.len() as i64);
        entry.set_perm(0o644).unwrap();
        archive.write_header(&entry).unwrap();

        // Use std::io::Write trait instead of write_data
        archive.write_all(data).unwrap();

        archive.finish().unwrap();
    }

    // Verify by reading back
    let mut archive = ReadArchive::open(&path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "hello.txt");

    let content = archive.read_data_to_vec().unwrap();
    assert_eq!(content, data);
}

#[test]
fn test_read_archive_io_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("io_read.tar");

    let data = b"Hello from io::Read!";

    // Create archive
    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .open_file(&path)
            .unwrap();
        archive.add_file("hello.txt", data).unwrap();
        archive.finish().unwrap();
    }

    // Read using io::Read trait
    let mut archive = ReadArchive::open(&path).unwrap();
    let _entry = archive.next_entry().unwrap().unwrap();

    let mut contents = Vec::new();
    archive.read_to_end(&mut contents).unwrap();
    assert_eq!(contents, data);
}

#[test]
fn test_io_copy_between_archives() {
    let dir = tempfile::tempdir().unwrap();
    let src_path = dir.path().join("source.tar");
    let dst_path = dir.path().join("dest.tar");

    let data = b"Data to be copied via io::copy";

    // Create source archive
    {
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .open_file(&src_path)
            .unwrap();
        archive.add_file("copied.txt", data).unwrap();
        archive.finish().unwrap();
    }

    // Copy entry data using io::copy
    {
        let mut src = ReadArchive::open(&src_path).unwrap();
        let src_entry = src.next_entry().unwrap().unwrap();
        let size = src_entry.size();
        let name = src_entry.pathname().unwrap();

        let mut dst = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .open_file(&dst_path)
            .unwrap();

        let mut entry = EntryMut::new();
        entry.set_pathname(&name).unwrap();
        entry.set_file_type(FileType::RegularFile);
        entry.set_size(size);
        entry.set_perm(0o644).unwrap();
        dst.write_header(&entry).unwrap();

        std::io::copy(&mut src, &mut dst).unwrap();
        dst.finish().unwrap();
    }

    // Verify destination
    let mut archive = ReadArchive::open(&dst_path).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "copied.txt");
    let content = archive.read_data_to_vec().unwrap();
    assert_eq!(content, data);
}

#[test]
fn test_callback_writer_composition() {
    // Demonstrates using CallbackWriter to pipe archive output through a custom Write.
    // Use std::io::Cursor which is 'static and owns its buffer.
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};

    let shared_buf = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let writer = shared_buf.clone();

    {
        // Wrap the shared cursor in a small adapter implementing Write
        struct SharedWriter(Arc<Mutex<Cursor<Vec<u8>>>>);
        impl Write for SharedWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().write(buf)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.0.lock().unwrap().flush()
            }
        }

        let callback = CallbackWriter::new(SharedWriter(writer));
        let mut archive = WriteArchive::new()
            .format(ArchiveFormat::TarPax)
            .compression(CompressionFormat::None)
            .open_callback(callback)
            .unwrap();

        archive
            .add_file("test.txt", b"callback writer content")
            .unwrap();
        archive.finish().unwrap();
    }

    // Extract the bytes and verify by reading
    let output_buf = shared_buf.lock().unwrap().get_ref().clone();
    let mut archive = ReadArchive::open_memory(&output_buf).unwrap();
    let entry = archive.next_entry().unwrap().unwrap();
    assert_eq!(entry.pathname().unwrap(), "test.txt");
    let content = archive.read_data_to_vec().unwrap();
    assert_eq!(content, b"callback writer content");
}
