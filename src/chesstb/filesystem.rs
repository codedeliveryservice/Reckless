//! File access: case-exact lookup and read-only mapping.

use std::path::Path;

/// Whether `path` exists spelled exactly as on disk (KrK is not KRK). The
/// directory is scanned only when the case-flipped name exists too.
pub fn file_exists_case_exact(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    let (Some(name), Some(dir)) = (path.file_name(), path.parent()) else {
        return false;
    };
    let name = name.to_string_lossy();
    let flipped: String = name
        .chars()
        .map(|c| if c.is_ascii_uppercase() { c.to_ascii_lowercase() } else { c.to_ascii_uppercase() })
        .collect();
    if flipped == name || !dir.join(&flipped).exists() {
        return true;
    }
    std::fs::read_dir(dir).is_ok_and(|entries| entries.flatten().any(|e| e.file_name().to_string_lossy() == name))
}

/// A table file, mapped read-only for random access. Unix and Windows map
/// files; elsewhere every open fails, so no table is found. A mapped file must
/// not change while it is open.
pub struct MappedFile {
    ptr: *const u8,
    len: usize,
}

// The mapping is read-only and lives as long as the value.
unsafe impl Send for MappedFile {}
unsafe impl Sync for MappedFile {}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    pub const PAGE_READONLY: u32 = 0x02;
    pub const FILE_MAP_READ: u32 = 0x04;
    pub const FILE_FLAG_RANDOM_ACCESS: u32 = 0x1000_0000;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn CreateFileMappingW(
            file: *mut c_void, attributes: *mut c_void, protect: u32, size_high: u32, size_low: u32, name: *const u16,
        ) -> *mut c_void;
        pub fn MapViewOfFile(
            mapping: *mut c_void, access: u32, offset_high: u32, offset_low: u32, bytes: usize,
        ) -> *mut c_void;
        pub fn UnmapViewOfFile(view: *const c_void) -> i32;
        pub fn CloseHandle(handle: *mut c_void) -> i32;
    }
}

impl MappedFile {
    #[cfg(unix)]
    pub fn open(path: &Path) -> Option<Self> {
        use std::os::unix::io::AsRawFd;

        let file = std::fs::File::open(path).ok()?;
        let len = usize::try_from(file.metadata().ok()?.len()).ok()?;
        if len == 0 {
            return None;
        }
        unsafe {
            let ptr = libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ, libc::MAP_PRIVATE, file.as_raw_fd(), 0);
            if ptr == libc::MAP_FAILED {
                return None;
            }
            libc::madvise(ptr, len, libc::MADV_RANDOM);
            Some(Self { ptr: ptr as *const u8, len })
        }
    }

    #[cfg(windows)]
    pub fn open(path: &Path) -> Option<Self> {
        use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};

        let file = std::fs::OpenOptions::new().read(true).custom_flags(win::FILE_FLAG_RANDOM_ACCESS).open(path).ok()?;
        let len = usize::try_from(file.metadata().ok()?.len()).ok()?;
        if len == 0 {
            return None;
        }
        unsafe {
            let mapping = win::CreateFileMappingW(
                file.as_raw_handle(),
                std::ptr::null_mut(),
                win::PAGE_READONLY,
                0,
                0,
                std::ptr::null(),
            );
            if mapping.is_null() {
                return None;
            }
            // The view keeps the mapping alive once its handle is closed.
            let ptr = win::MapViewOfFile(mapping, win::FILE_MAP_READ, 0, 0, 0);
            win::CloseHandle(mapping);
            if ptr.is_null() {
                return None;
            }
            Some(Self { ptr: ptr as *const u8, len })
        }
    }

    #[cfg(not(any(unix, windows)))]
    pub fn open(_: &Path) -> Option<Self> {
        None
    }

    pub fn data(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for MappedFile {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.len);
        }
        #[cfg(windows)]
        unsafe {
            win::UnmapViewOfFile(self.ptr.cast());
        }
    }
}
