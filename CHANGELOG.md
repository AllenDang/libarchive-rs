# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-10-16

### Added

- New `ReadArchive::open_filenames_with_passphrase()` method for reading encrypted multi-volume archives
  - Combines multi-volume archive support with password protection
  - Supports encrypted RAR archives split into multiple parts
  - Works with any archive format that supports both encryption and multi-volume features
- New example `read_encrypted_multivolume.rs` demonstrating encrypted multi-volume archive extraction
- Comprehensive test coverage for the new encrypted multi-volume functionality

### Changed

- N/A

### Deprecated

- N/A

### Removed

- N/A

### Fixed

- N/A

### Security

- N/A

## [0.1.0] - Previous Release

Initial release with basic libarchive functionality including:
- Archive reading and writing
- Multi-volume archive support
- Password-protected archive support
- Multiple compression formats
- Sparse file support
