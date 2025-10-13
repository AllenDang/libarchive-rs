//! Custom callback support for reading and writing archives
//!
//! This module provides callback-based interfaces for streaming data and
//! tracking progress during archive operations.

use std::ffi::c_void;
use std::io::{Read, Write};
use std::os::raw::c_int;
use std::sync::Mutex;

/// Type for callback cleanup function
pub(crate) type DropFn = unsafe fn(*mut c_void);

/// Internal state for read callbacks
struct ReadCallbackState<R: Read> {
    reader: R,
    buffer: Vec<u8>,
}

/// Internal state for write callbacks
struct WriteCallbackState<W: Write> {
    writer: W,
}

/// Trait for progress tracking callbacks
///
/// Implement this trait to receive progress notifications during archive operations.
pub trait ProgressCallback {
    /// Called when progress is made reading or writing an entry
    ///
    /// # Arguments
    ///
    /// * `bytes_processed` - Total bytes processed so far
    /// * `total_bytes` - Total bytes to process (may be 0 if unknown)
    fn on_progress(&mut self, bytes_processed: u64, total_bytes: u64);
}

/// C callback function for reading data
///
/// # Safety
/// This function is called by libarchive from C. The client_data pointer must be
/// a valid pointer to a Mutex<ReadCallbackState<R>> that was created by this module.
unsafe extern "C" fn read_callback_impl<R: Read>(
    _archive: *mut libarchive2_sys::archive,
    client_data: *mut c_void,
    buffer: *mut *const c_void,
) -> isize {
    if client_data.is_null() || buffer.is_null() {
        return -1;
    }

    // SAFETY: client_data is a valid pointer to Mutex<ReadCallbackState<R>>
    // created by CallbackReader::into_raw_parts. It remains valid until
    // the drop_fn is called.
    unsafe {
        let state = &*(client_data as *mut Mutex<ReadCallbackState<R>>);
        let mut guard = match state.lock() {
            Ok(g) => g,
            Err(_) => return -1, // Poisoned mutex
        };

        // Get mutable references to both fields
        let ReadCallbackState {
            reader,
            buffer: buf,
        } = &mut *guard;

        // Read into buffer
        let result = reader.read(buf);

        match result {
            Ok(n) => {
                *buffer = buf.as_ptr() as *const c_void;
                n as isize
            }
            Err(_) => -1,
        }
    }
}

/// C callback function for writing data
///
/// # Safety
/// This function is called by libarchive from C. The client_data pointer must be
/// a valid pointer to a Mutex<WriteCallbackState<W>> that was created by this module.
unsafe extern "C" fn write_callback_impl<W: Write>(
    _archive: *mut libarchive2_sys::archive,
    client_data: *mut c_void,
    buffer: *const c_void,
    length: usize,
) -> isize {
    if client_data.is_null() || buffer.is_null() {
        return -1;
    }

    // SAFETY: client_data is a valid pointer to Mutex<WriteCallbackState<W>>
    // created by CallbackWriter::into_raw_parts. It remains valid until
    // the drop_fn is called.
    unsafe {
        let state = &*(client_data as *mut Mutex<WriteCallbackState<W>>);
        let mut guard = match state.lock() {
            Ok(g) => g,
            Err(_) => return -1, // Poisoned mutex
        };

        let data = std::slice::from_raw_parts(buffer as *const u8, length);
        match guard.writer.write_all(data) {
            Ok(()) => length as isize,
            Err(_) => -1,
        }
    }
}

/// C callback function for closing (no-op)
unsafe extern "C" fn close_callback_impl(
    _archive: *mut libarchive2_sys::archive,
    _client_data: *mut c_void,
) -> c_int {
    0
}

/// Builder for reading archives with custom Read implementations
pub struct CallbackReader<R: Read> {
    state: Box<Mutex<ReadCallbackState<R>>>,
}

impl<R: Read> CallbackReader<R> {
    /// Create a new callback reader from any type implementing Read
    ///
    /// The reader is wrapped in a Mutex to ensure thread-safe access if
    /// libarchive calls the callback from multiple threads.
    pub fn new(reader: R) -> Self {
        const BUFFER_SIZE: usize = 65536; // 64KB buffer
        CallbackReader {
            state: Box::new(Mutex::new(ReadCallbackState {
                reader,
                buffer: vec![0u8; BUFFER_SIZE],
            })),
        }
    }

    pub(crate) fn into_raw_parts(self) -> (*mut c_void, *const c_void, *const c_void, DropFn) {
        let ptr = Box::into_raw(self.state) as *mut c_void;

        // Create a properly typed drop function for this specific type
        unsafe fn drop_fn<R: Read>(ptr: *mut c_void) {
            // SAFETY: ptr was created by Box::into_raw in into_raw_parts
            unsafe {
                let _ = Box::from_raw(ptr as *mut Mutex<ReadCallbackState<R>>);
            }
        }

        // Return function pointers as void pointers to avoid type issues
        (
            ptr,
            read_callback_impl::<R> as *const c_void,
            close_callback_impl as *const c_void,
            drop_fn::<R>,
        )
    }
}

/// Builder for writing archives with custom Write implementations
pub struct CallbackWriter<W: Write> {
    state: Box<Mutex<WriteCallbackState<W>>>,
}

impl<W: Write> CallbackWriter<W> {
    /// Create a new callback writer from any type implementing Write
    ///
    /// The writer is wrapped in a Mutex to ensure thread-safe access if
    /// libarchive calls the callback from multiple threads.
    pub fn new(writer: W) -> Self {
        CallbackWriter {
            state: Box::new(Mutex::new(WriteCallbackState { writer })),
        }
    }

    pub(crate) fn into_raw_parts(self) -> (*mut c_void, *const c_void, *const c_void, DropFn) {
        let ptr = Box::into_raw(self.state) as *mut c_void;

        // Create a properly typed drop function for this specific type
        unsafe fn drop_fn<W: Write>(ptr: *mut c_void) {
            // SAFETY: ptr was created by Box::into_raw in into_raw_parts
            unsafe {
                let _ = Box::from_raw(ptr as *mut Mutex<WriteCallbackState<W>>);
            }
        }

        // Return function pointers as void pointers to avoid type issues
        (
            ptr,
            write_callback_impl::<W> as *const c_void,
            close_callback_impl as *const c_void,
            drop_fn::<W>,
        )
    }
}

/// Progress tracker for monitoring archive operations
pub struct ProgressTracker {
    callback: Box<dyn ProgressCallback>,
    bytes_processed: u64,
    total_bytes: u64,
}

impl ProgressTracker {
    /// Create a new progress tracker with a callback
    pub fn new<C: ProgressCallback + 'static>(callback: C) -> Self {
        ProgressTracker {
            callback: Box::new(callback),
            bytes_processed: 0,
            total_bytes: 0,
        }
    }

    /// Update progress
    pub fn update(&mut self, bytes: u64) {
        self.bytes_processed += bytes;
        self.callback
            .on_progress(self.bytes_processed, self.total_bytes);
    }

    /// Set total bytes
    pub fn set_total(&mut self, total: u64) {
        self.total_bytes = total;
    }

    /// Reset progress
    pub fn reset(&mut self) {
        self.bytes_processed = 0;
    }
}
