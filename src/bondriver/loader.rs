//! BonDriver DLL loader

use std::path::Path;

use libloading::{Library, Symbol};

use crate::error::{BonrecError, Result};

use super::ffi::{CreateBonDriverFn, IBonDriver2};

/// BonDriver DLL loader
pub struct BonDriverLoader {
    /// Loaded library (kept alive for symbol validity)
    _library: Library,
    /// Pointer to IBonDriver2 interface
    driver: *mut IBonDriver2,
}

// Safety: BonDriver is designed to be used from single thread
// We ensure proper synchronization at higher level
unsafe impl Send for BonDriverLoader {}

impl BonDriverLoader {
    /// Load a BonDriver DLL from the specified path
    ///
    /// # Arguments
    /// * `path` - Path to the BonDriver DLL file
    ///
    /// # Errors
    /// Returns error if DLL cannot be loaded or CreateBonDriver fails
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Check if file exists
        if !path.exists() {
            return Err(BonrecError::BonDriverNotFound {
                path: path.to_path_buf(),
            });
        }

        // Load the DLL
        let library = unsafe {
            Library::new(path).map_err(|e| BonrecError::dll_load_error(path, e))?
        };

        // Get CreateBonDriver function
        let create_fn: Symbol<CreateBonDriverFn> = unsafe {
            library.get(b"CreateBonDriver").map_err(|e| {
                BonrecError::dll_load_error(path, e)
            })?
        };

        // Call CreateBonDriver to get interface pointer
        let driver = unsafe { create_fn() as *mut IBonDriver2 };

        if driver.is_null() {
            return Err(BonrecError::CreateBonDriverFailed {
                path: path.to_path_buf(),
            });
        }

        log::debug!("Loaded BonDriver: {}", path.display());

        Ok(BonDriverLoader {
            _library: library,
            driver,
        })
    }

    /// Get the raw IBonDriver2 pointer
    ///
    /// # Safety
    /// The caller must ensure the pointer is used correctly according to
    /// the BonDriver interface specification.
    pub fn driver_ptr(&self) -> *mut IBonDriver2 {
        self.driver
    }
}

impl Drop for BonDriverLoader {
    fn drop(&mut self) {
        if !self.driver.is_null() {
            unsafe {
                let vtable = &*(*self.driver).vtable;
                (vtable.release)(self.driver);
            }
            log::debug!("Released BonDriver");
        }
    }
}
