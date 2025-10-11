//! Error types for libarchive operations

use std::ffi::CStr;
use std::fmt;

/// Result type for libarchive operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for libarchive operations
#[derive(Debug)]
pub enum Error {
    /// Error from libarchive with error message
    Archive {
        /// Error code from libarchive
        code: i32,
        /// Error message from libarchive
        message: String,
    },
    /// UTF-8 conversion error
    Utf8(std::str::Utf8Error),
    /// I/O error
    Io(std::io::Error),
    /// Null pointer error
    NullPointer,
    /// Invalid argument
    InvalidArgument(String),
}

impl Error {
    /// Create an error from a libarchive archive pointer
    pub(crate) unsafe fn from_archive(archive: *mut libarchive2_sys::archive) -> Self {
        // SAFETY: Caller must ensure archive is a valid pointer
        unsafe {
            let code = libarchive2_sys::archive_errno(archive);
            let msg_ptr = libarchive2_sys::archive_error_string(archive);
            let message = if msg_ptr.is_null() {
                format!("Unknown error (code: {})", code)
            } else {
                CStr::from_ptr(msg_ptr).to_string_lossy().into_owned()
            };

            Error::Archive { code, message }
        }
    }

    /// Check a return code from libarchive and convert to Result
    pub(crate) unsafe fn from_return_code(
        ret: i32,
        archive: *mut libarchive2_sys::archive,
    ) -> Result<i32> {
        if ret < 0 {
            // SAFETY: Caller must ensure archive is a valid pointer
            Err(unsafe { Self::from_archive(archive) })
        } else {
            Ok(ret)
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Archive { code, message } => {
                write!(f, "libarchive error (code {}): {}", code, message)
            }
            Error::Utf8(e) => write!(f, "UTF-8 conversion error: {}", e),
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::NullPointer => write!(f, "Null pointer error"),
            Error::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Utf8(e) => Some(e),
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Self {
        Error::Utf8(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
