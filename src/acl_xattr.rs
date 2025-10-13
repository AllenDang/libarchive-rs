//! ACL (Access Control List) and Extended Attributes support
//!
//! This module provides APIs for reading and manipulating ACLs and extended attributes
//! on archive entries.

use crate::entry::{Entry, EntryMut};
use crate::error::{Error, Result};
use std::ffi::{CStr, CString};

/// ACL entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclType {
    /// Access ACL
    Access,
    /// Default ACL
    Default,
}

/// ACL permission flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AclPermissions {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
}

impl AclPermissions {
    /// Create permissions from bitmask
    pub fn from_bits(bits: i32) -> Self {
        const PERM_READ: i32 = 0x04;
        const PERM_WRITE: i32 = 0x02;
        const PERM_EXEC: i32 = 0x01;

        AclPermissions {
            read: (bits & PERM_READ) != 0,
            write: (bits & PERM_WRITE) != 0,
            execute: (bits & PERM_EXEC) != 0,
        }
    }

    /// Convert permissions to bitmask
    pub fn to_bits(&self) -> i32 {
        const PERM_READ: i32 = 0x04;
        const PERM_WRITE: i32 = 0x02;
        const PERM_EXEC: i32 = 0x01;

        let mut bits = 0;
        if self.read {
            bits |= PERM_READ;
        }
        if self.write {
            bits |= PERM_WRITE;
        }
        if self.execute {
            bits |= PERM_EXEC;
        }
        bits
    }
}

/// ACL tag type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclTag {
    /// User owner
    User,
    /// Group owner
    Group,
    /// Other
    Other,
    /// Mask
    Mask,
    /// Named user
    NamedUser,
    /// Named group
    NamedGroup,
}

/// ACL entry
#[derive(Debug, Clone)]
pub struct AclEntry {
    /// ACL type (access or default)
    pub acl_type: AclType,
    /// Tag type
    pub tag: AclTag,
    /// Permissions
    pub permissions: AclPermissions,
    /// Name (for named user/group)
    pub name: Option<String>,
    /// ID (for named user/group)
    pub id: Option<i32>,
}

/// Extended attribute (xattr)
#[derive(Debug, Clone)]
pub struct Xattr {
    /// Attribute name
    pub name: String,
    /// Attribute value
    pub value: Vec<u8>,
}

/// Extension trait for Entry to add ACL/xattr reading
pub trait EntryAclExt {
    /// Get the ACL text representation
    fn acl_text(&self) -> Option<String>;

    /// Check if entry has ACLs
    fn has_acl(&self) -> bool;

    /// Get count of ACL entries
    fn acl_count(&self) -> usize;

    /// Get count of extended attributes
    fn xattr_count(&self) -> usize;

    /// Get all extended attributes
    fn xattrs(&self) -> Vec<Xattr>;
}

impl<'a> EntryAclExt for Entry<'a> {
    fn acl_text(&self) -> Option<String> {
        unsafe {
            // Get ACL text for ACCESS ACLs
            let ptr = libarchive2_sys::archive_entry_acl_to_text(
                self.entry,
                std::ptr::null_mut(),
                libarchive2_sys::ARCHIVE_ENTRY_ACL_TYPE_ACCESS as i32,
            );
            if ptr.is_null() {
                None
            } else {
                let text = CStr::from_ptr(ptr).to_string_lossy().into_owned();
                // SAFETY: According to libarchive documentation, archive_entry_acl_to_text
                // returns a pointer to memory managed by the archive_entry.
                // The string is valid until the entry is freed or ACL is modified.
                // We copy it to an owned String, so no manual free is needed.
                Some(text)
            }
        }
    }

    fn has_acl(&self) -> bool {
        self.acl_count() > 0
    }

    fn acl_count(&self) -> usize {
        unsafe {
            libarchive2_sys::archive_entry_acl_reset(
                self.entry,
                libarchive2_sys::ARCHIVE_ENTRY_ACL_TYPE_ACCESS as i32,
            );
            let mut count = 0;
            loop {
                let ret = libarchive2_sys::archive_entry_acl_next(
                    self.entry,
                    libarchive2_sys::ARCHIVE_ENTRY_ACL_TYPE_ACCESS as i32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if ret != libarchive2_sys::ARCHIVE_OK as i32 {
                    break;
                }
                count += 1;
            }
            count
        }
    }

    fn xattr_count(&self) -> usize {
        unsafe {
            libarchive2_sys::archive_entry_xattr_reset(self.entry);
            let mut count = 0;
            loop {
                let ret = libarchive2_sys::archive_entry_xattr_next(
                    self.entry,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if ret != libarchive2_sys::ARCHIVE_OK as i32 {
                    break;
                }
                count += 1;
            }
            count
        }
    }

    fn xattrs(&self) -> Vec<Xattr> {
        unsafe {
            let mut xattrs = Vec::new();
            libarchive2_sys::archive_entry_xattr_reset(self.entry);

            loop {
                let mut name_ptr: *const std::os::raw::c_char = std::ptr::null();
                let mut value_ptr: *const std::os::raw::c_void = std::ptr::null();
                let mut size: usize = 0;

                let ret = libarchive2_sys::archive_entry_xattr_next(
                    self.entry,
                    &mut name_ptr,
                    &mut value_ptr,
                    &mut size,
                );

                if ret != libarchive2_sys::ARCHIVE_OK as i32 {
                    break;
                }

                if !name_ptr.is_null() {
                    let name = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();
                    let value = if !value_ptr.is_null() && size > 0 {
                        std::slice::from_raw_parts(value_ptr as *const u8, size).to_vec()
                    } else {
                        Vec::new()
                    };

                    xattrs.push(Xattr { name, value });
                }
            }

            xattrs
        }
    }
}

/// Extension trait for EntryMut to add ACL/xattr manipulation
pub trait EntryMutAclExt {
    /// Clear all ACLs
    fn clear_acl(&mut self);

    /// Add an ACL entry from text
    fn add_acl_text(&mut self, text: &str, acl_type: AclType) -> Result<()>;

    /// Add an ACL entry
    fn add_acl_entry(
        &mut self,
        acl_type: AclType,
        tag: AclTag,
        permissions: AclPermissions,
        name: Option<&str>,
        id: Option<i32>,
    ) -> Result<()>;

    /// Clear all extended attributes
    fn clear_xattrs(&mut self);

    /// Add an extended attribute
    fn add_xattr(&mut self, name: &str, value: &[u8]) -> Result<()>;
}

impl EntryMutAclExt for EntryMut {
    fn clear_acl(&mut self) {
        unsafe {
            libarchive2_sys::archive_entry_acl_clear(self.entry);
        }
    }

    fn add_acl_text(&mut self, text: &str, acl_type: AclType) -> Result<()> {
        let c_text = CString::new(text)
            .map_err(|_| Error::InvalidArgument("ACL text contains null byte".to_string()))?;

        let type_flag = match acl_type {
            AclType::Access => libarchive2_sys::ARCHIVE_ENTRY_ACL_TYPE_ACCESS,
            AclType::Default => libarchive2_sys::ARCHIVE_ENTRY_ACL_TYPE_DEFAULT,
        };

        unsafe {
            let ret = libarchive2_sys::archive_entry_acl_from_text(
                self.entry,
                c_text.as_ptr(),
                type_flag as i32,
            );
            if ret != libarchive2_sys::ARCHIVE_OK as i32 {
                return Err(Error::InvalidArgument(
                    "Failed to parse ACL text".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn add_acl_entry(
        &mut self,
        acl_type: AclType,
        tag: AclTag,
        permissions: AclPermissions,
        name: Option<&str>,
        id: Option<i32>,
    ) -> Result<()> {
        let type_flag = match acl_type {
            AclType::Access => libarchive2_sys::ARCHIVE_ENTRY_ACL_TYPE_ACCESS,
            AclType::Default => libarchive2_sys::ARCHIVE_ENTRY_ACL_TYPE_DEFAULT,
        };

        let tag_flag = match tag {
            AclTag::User => libarchive2_sys::ARCHIVE_ENTRY_ACL_USER_OBJ,
            AclTag::Group => libarchive2_sys::ARCHIVE_ENTRY_ACL_GROUP_OBJ,
            AclTag::Other => libarchive2_sys::ARCHIVE_ENTRY_ACL_OTHER,
            AclTag::Mask => libarchive2_sys::ARCHIVE_ENTRY_ACL_MASK,
            AclTag::NamedUser => libarchive2_sys::ARCHIVE_ENTRY_ACL_USER,
            AclTag::NamedGroup => libarchive2_sys::ARCHIVE_ENTRY_ACL_GROUP,
        };

        let name_cstr = if let Some(n) = name {
            Some(
                CString::new(n)
                    .map_err(|_| Error::InvalidArgument("Name contains null byte".to_string()))?,
            )
        } else {
            None
        };

        unsafe {
            libarchive2_sys::archive_entry_acl_add_entry(
                self.entry,
                type_flag as i32,
                permissions.to_bits(),
                tag_flag as i32,
                id.unwrap_or(-1),
                name_cstr.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            );
        }

        Ok(())
    }

    fn clear_xattrs(&mut self) {
        unsafe {
            libarchive2_sys::archive_entry_xattr_clear(self.entry);
        }
    }

    fn add_xattr(&mut self, name: &str, value: &[u8]) -> Result<()> {
        let c_name = CString::new(name)
            .map_err(|_| Error::InvalidArgument("Xattr name contains null byte".to_string()))?;

        unsafe {
            libarchive2_sys::archive_entry_xattr_add_entry(
                self.entry,
                c_name.as_ptr(),
                value.as_ptr() as *const std::os::raw::c_void,
                value.len(),
            );
        }

        Ok(())
    }
}
