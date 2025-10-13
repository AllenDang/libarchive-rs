//! Archive matching and filtering API
//!
//! This module provides pattern-based filtering for archive entries,
//! allowing you to include or exclude specific files based on patterns.

use crate::entry::Entry;
use crate::error::{Error, Result};
use std::ffi::CString;

/// Archive matcher for filtering entries based on patterns
///
/// # Thread Safety
///
/// `ArchiveMatch` is `Send` but not `Sync`. You can transfer ownership between threads,
/// but cannot share references across threads.
pub struct ArchiveMatch {
    matcher: *mut libarchive2_sys::archive,
}

// SAFETY: ArchiveMatch can be sent between threads because the matcher pointer
// is owned exclusively by this instance and libarchive match objects
// can be used from different threads (just not concurrently).
unsafe impl Send for ArchiveMatch {}

// Note: ArchiveMatch is NOT Sync because libarchive matchers are not thread-safe
// for concurrent access.

impl ArchiveMatch {
    /// Create a new archive matcher
    pub fn new() -> Result<Self> {
        unsafe {
            let matcher = libarchive2_sys::archive_match_new();
            if matcher.is_null() {
                return Err(Error::NullPointer);
            }
            Ok(ArchiveMatch { matcher })
        }
    }

    /// Include entries matching a shell-style pattern
    ///
    /// Patterns may contain wildcards:
    /// - `*` matches any sequence of characters
    /// - `?` matches any single character
    /// - `[...]` matches any character in the brackets
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ArchiveMatch;
    ///
    /// let mut matcher = ArchiveMatch::new()?;
    /// matcher.include_pattern("*.txt")?;  // Include all .txt files
    /// matcher.include_pattern("docs/*")?;  // Include all files in docs/
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn include_pattern(&mut self, pattern: &str) -> Result<()> {
        let c_pattern = CString::new(pattern)
            .map_err(|_| Error::InvalidArgument("Pattern contains null byte".to_string()))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_match_include_pattern(self.matcher, c_pattern.as_ptr()),
                self.matcher,
            )?;
        }
        Ok(())
    }

    /// Exclude entries matching a shell-style pattern
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ArchiveMatch;
    ///
    /// let mut matcher = ArchiveMatch::new()?;
    /// matcher.exclude_pattern("*.o")?;     // Exclude object files
    /// matcher.exclude_pattern("*.tmp")?;   // Exclude temp files
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn exclude_pattern(&mut self, pattern: &str) -> Result<()> {
        let c_pattern = CString::new(pattern)
            .map_err(|_| Error::InvalidArgument("Pattern contains null byte".to_string()))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_match_exclude_pattern(self.matcher, c_pattern.as_ptr()),
                self.matcher,
            )?;
        }
        Ok(())
    }

    /// Include entries matching a specific pathname
    ///
    /// This is for exact pathname matches, not patterns.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ArchiveMatch;
    ///
    /// let mut matcher = ArchiveMatch::new()?;
    /// matcher.include_pathname("README.md")?;
    /// matcher.include_pathname("src/main.rs")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn include_pathname(&mut self, pathname: &str) -> Result<()> {
        let c_pathname = CString::new(pathname)
            .map_err(|_| Error::InvalidArgument("Pathname contains null byte".to_string()))?;

        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_match_include_file_time(
                    self.matcher,
                    libarchive2_sys::ARCHIVE_MATCH_MTIME as i32,
                    c_pathname.as_ptr(),
                ),
                self.matcher,
            )?;
        }
        Ok(())
    }

    /// Exclude entries matching a specific pathname
    ///
    /// **Note**: This method is not yet fully implemented. Use `exclude_pattern()`
    /// for pattern-based exclusions instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ArchiveMatch;
    ///
    /// let mut matcher = ArchiveMatch::new()?;
    /// // Use exclude_pattern instead:
    /// matcher.exclude_pattern(".DS_Store")?;
    /// matcher.exclude_pattern("thumbs.db")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn exclude_pathname(&mut self, _pathname: &str) -> Result<()> {
        Err(Error::InvalidArgument(
            "exclude_pathname is not yet implemented. Use exclude_pattern() instead.".to_string(),
        ))
    }

    /// Include only entries newer than the specified time (in seconds since epoch)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ArchiveMatch;
    /// use std::time::{SystemTime, UNIX_EPOCH};
    ///
    /// let mut matcher = ArchiveMatch::new()?;
    /// let one_week_ago = SystemTime::now()
    ///     .duration_since(UNIX_EPOCH)?
    ///     .as_secs() as i64 - (7 * 24 * 60 * 60);
    /// matcher.include_time_newer_than(one_week_ago, 0)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn include_time_newer_than(&mut self, sec: i64, nsec: i64) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_match_include_time(
                    self.matcher,
                    libarchive2_sys::ARCHIVE_MATCH_NEWER as i32,
                    sec,
                    nsec,
                ),
                self.matcher,
            )?;
        }
        Ok(())
    }

    /// Include only entries older than the specified time (in seconds since epoch)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ArchiveMatch;
    /// use std::time::{SystemTime, UNIX_EPOCH};
    ///
    /// let mut matcher = ArchiveMatch::new()?;
    /// let one_year_ago = SystemTime::now()
    ///     .duration_since(UNIX_EPOCH)?
    ///     .as_secs() as i64 - (365 * 24 * 60 * 60);
    /// matcher.include_time_older_than(one_year_ago, 0)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn include_time_older_than(&mut self, sec: i64, nsec: i64) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_match_include_time(
                    self.matcher,
                    libarchive2_sys::ARCHIVE_MATCH_OLDER as i32,
                    sec,
                    nsec,
                ),
                self.matcher,
            )?;
        }
        Ok(())
    }

    /// Include only entries with owner matching the specified UID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ArchiveMatch;
    ///
    /// let mut matcher = ArchiveMatch::new()?;
    /// matcher.include_uid(1000)?;  // Include only files owned by UID 1000
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn include_uid(&mut self, uid: i64) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_match_include_uid(self.matcher, uid),
                self.matcher,
            )?;
        }
        Ok(())
    }

    /// Include only entries with group matching the specified GID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::ArchiveMatch;
    ///
    /// let mut matcher = ArchiveMatch::new()?;
    /// matcher.include_gid(1000)?;  // Include only files owned by GID 1000
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn include_gid(&mut self, gid: i64) -> Result<()> {
        unsafe {
            Error::from_return_code(
                libarchive2_sys::archive_match_include_gid(self.matcher, gid),
                self.matcher,
            )?;
        }
        Ok(())
    }

    /// Exclude entries with owner matching the specified UID
    ///
    /// **Note**: This method is not yet implemented as libarchive doesn't provide
    /// a direct API for excluding UIDs. Use custom filtering in your application code.
    pub fn exclude_uid(&mut self, _uid: i64) -> Result<()> {
        Err(Error::InvalidArgument(
            "exclude_uid is not supported by libarchive. Implement custom filtering.".to_string(),
        ))
    }

    /// Exclude entries with group matching the specified GID
    ///
    /// **Note**: This method is not yet implemented as libarchive doesn't provide
    /// a direct API for excluding GIDs. Use custom filtering in your application code.
    pub fn exclude_gid(&mut self, _gid: i64) -> Result<()> {
        Err(Error::InvalidArgument(
            "exclude_gid is not supported by libarchive. Implement custom filtering.".to_string(),
        ))
    }

    /// Check if an entry matches the configured filters
    ///
    /// Returns `true` if the entry should be included based on the configured patterns.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libarchive2::{ReadArchive, ArchiveMatch};
    ///
    /// let mut archive = ReadArchive::open("archive.tar.gz")?;
    /// let mut matcher = ArchiveMatch::new()?;
    /// matcher.include_pattern("*.txt")?;
    ///
    /// while let Some(entry) = archive.next_entry()? {
    ///     if matcher.matches(&entry)? {
    ///         println!("Matched: {}", entry.pathname().unwrap_or_default());
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn matches(&mut self, entry: &Entry) -> Result<bool> {
        unsafe {
            let ret = libarchive2_sys::archive_match_excluded(self.matcher, entry.entry);
            if ret < 0 {
                // Error occurred
                Err(Error::from_archive(self.matcher))
            } else {
                // 0 = not excluded (matches), >0 = excluded
                Ok(ret == 0)
            }
        }
    }

    /// Check if an entry is time-excluded based on the configured time filters
    pub fn time_excluded(&mut self, entry: &Entry) -> Result<bool> {
        unsafe {
            let ret = libarchive2_sys::archive_match_time_excluded(self.matcher, entry.entry);
            Ok(ret != 0)
        }
    }

    /// Check if an entry's path is excluded based on the configured patterns
    pub fn path_excluded(&mut self, entry: &Entry) -> Result<bool> {
        unsafe {
            let ret = libarchive2_sys::archive_match_path_excluded(self.matcher, entry.entry);
            Ok(ret != 0)
        }
    }
}

impl Drop for ArchiveMatch {
    fn drop(&mut self) {
        unsafe {
            if !self.matcher.is_null() {
                libarchive2_sys::archive_match_free(self.matcher);
            }
        }
    }
}

// Note: Default implementation removed because matcher creation can fail.
// Use ArchiveMatch::new() instead.
