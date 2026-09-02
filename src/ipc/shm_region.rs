use std::ffi::CString;
use std::ptr::{self, NonNull};
use crate::error::CscpError;

/// Owns an mmap'd POSIX shared memory region.
/// 
/// The `is_owner` flag controls whether `shm_unlink` is called on Drop.
/// OmniSim (EnvSide) is always the owner. The Controller (CtrlSide) attaches
/// without ownership.
pub struct ShmRegion {
    ptr: NonNull<u8>,
    size: usize,
    name: CString,
    is_owner: bool,
}

// SAFETY: The mmap pointer is process-shared by design.
unsafe impl Send for ShmRegion {}
unsafe impl Sync for ShmRegion {}

impl ShmRegion {
    /// Creates a new POSIX shared memory region and owns it.
    pub fn create(name: &str, size: usize) -> Result<Self, CscpError> {
        let name_str = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{}", name)
        };
        let c_name = CString::new(name_str).map_err(|_| CscpError::ShmOpenFailed("Invalid SHM name".into()))?;

        unsafe {
            // SAFETY: c_name is a valid CString. We ignore the error as this is just cleanup for stale regions.
            libc::shm_unlink(c_name.as_ptr());

            // SAFETY: c_name is a valid CString.
            let fd = libc::shm_open(
                c_name.as_ptr(),
                libc::O_CREAT | libc::O_RDWR | libc::O_EXCL,
                0o600,
            );
            if fd < 0 {
                return Err(CscpError::ShmOpenFailed(std::io::Error::last_os_error().to_string()));
            }

            // SAFETY: fd is valid.
            if libc::ftruncate(fd, size as libc::off_t) < 0 {
                let err = std::io::Error::last_os_error().to_string();
                libc::close(fd);
                return Err(CscpError::ShmOpenFailed(err));
            }

            // SAFETY: fd is valid and size is correct.
            let ptr = libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            );

            if ptr == libc::MAP_FAILED {
                let err = std::io::Error::last_os_error().to_string();
                libc::close(fd);
                return Err(CscpError::ShmMapFailed(err));
            }

            // SAFETY: fd is no longer needed after mmap.
            libc::close(fd);

            // SAFETY: ptr is valid and mapped PROT_WRITE. size is exactly the size of the region.
            ptr::write_bytes(ptr as *mut u8, 0, size);

            Ok(ShmRegion {
                ptr: NonNull::new(ptr as *mut u8).unwrap(),
                size,
                name: c_name,
                is_owner: true,
            })
        }
    }

    /// Attaches to an existing POSIX shared memory region.
    pub fn attach(name: &str, size: usize) -> Result<Self, CscpError> {
        let name_str = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{}", name)
        };
        let c_name = CString::new(name_str).map_err(|_| CscpError::ShmOpenFailed("Invalid SHM name".into()))?;

        unsafe {
            // SAFETY: c_name is a valid CString.
            let fd = libc::shm_open(
                c_name.as_ptr(),
                libc::O_RDWR,
                0,
            );
            if fd < 0 {
                return Err(CscpError::ShmOpenFailed(std::io::Error::last_os_error().to_string()));
            }

            // SAFETY: fd is valid.
            let ptr = libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            );

            if ptr == libc::MAP_FAILED {
                let err = std::io::Error::last_os_error().to_string();
                libc::close(fd);
                return Err(CscpError::ShmMapFailed(err));
            }

            // SAFETY: fd is no longer needed after mmap.
            libc::close(fd);

            Ok(ShmRegion {
                ptr: NonNull::new(ptr as *mut u8).unwrap(),
                size,
                name: c_name,
                is_owner: false,
            })
        }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_owner(&self) -> bool {
        self.is_owner
    }

    pub fn name(&self) -> &str {
        self.name.to_str().unwrap_or("<invalid>")
    }
}

impl Drop for ShmRegion {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: ptr and size were returned by a successful mmap call
            libc::munmap(self.ptr.as_ptr() as *mut libc::c_void, self.size);
            
            if self.is_owner {
                // SAFETY: name is a valid CString from construction
                libc::shm_unlink(self.name.as_ptr());
            }
        }
    }
}
