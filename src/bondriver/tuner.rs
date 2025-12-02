//! BonDriver tuner operations

use std::path::Path;

use crate::error::{BonrecError, Result};

use super::ffi::{lpctstr_to_string, FALSE, TRUE};
use super::loader::BonDriverLoader;

/// TS packet size (188 bytes)
pub const TS_PACKET_SIZE: usize = 188;

/// Default TS buffer size (188 * 256 = ~48KB)
pub const DEFAULT_TS_BUFFER_SIZE: usize = TS_PACKET_SIZE * 256;

/// BonDriver tuner wrapper with RAII
pub struct Tuner {
    loader: BonDriverLoader,
    tuner_opened: bool,
    dll_path: String,
}

impl Tuner {
    /// Create a new tuner from a BonDriver DLL path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let loader = BonDriverLoader::load(path)?;

        Ok(Tuner {
            loader,
            tuner_opened: false,
            dll_path: path.to_string_lossy().into_owned(),
        })
    }

    /// Get the DLL path
    pub fn dll_path(&self) -> &str {
        &self.dll_path
    }

    /// Open the tuner
    pub fn open(&mut self) -> Result<()> {
        if self.tuner_opened {
            return Ok(());
        }

        let driver = self.loader.driver_ptr();
        let result = unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.open_tuner)(driver)
        };

        if result == FALSE {
            return Err(BonrecError::TunerOpenFailed);
        }

        self.tuner_opened = true;
        log::info!("Tuner opened: {}", self.dll_path);
        Ok(())
    }

    /// Close the tuner
    pub fn close(&mut self) {
        if !self.tuner_opened {
            return;
        }

        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.close_tuner)(driver);
        }

        self.tuner_opened = false;
        log::info!("Tuner closed: {}", self.dll_path);
    }

    /// Check if tuner is opened
    pub fn is_opened(&self) -> bool {
        self.tuner_opened
    }

    /// Get tuner name
    pub fn get_tuner_name(&self) -> Option<String> {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            let name_ptr = (vtable.get_tuner_name)(driver);
            lpctstr_to_string(name_ptr)
        }
    }

    /// Check if tuner is opening
    pub fn is_tuner_opening(&self) -> bool {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.is_tuner_opening)(driver) == TRUE
        }
    }

    /// Set channel
    pub fn set_channel(&mut self, space: u32, channel: u32) -> Result<()> {
        let driver = self.loader.driver_ptr();
        let result = unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.set_channel)(driver, space, channel)
        };

        if result == FALSE {
            return Err(BonrecError::SetChannelFailed { space, channel });
        }

        log::debug!("Set channel: space={}, channel={}", space, channel);
        Ok(())
    }

    /// Get current space index
    pub fn get_cur_space(&self) -> u32 {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.get_cur_space)(driver)
        }
    }

    /// Get current channel index
    pub fn get_cur_channel(&self) -> u32 {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.get_cur_channel)(driver)
        }
    }

    /// Get signal level in dB
    pub fn get_signal_level(&self) -> f32 {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.get_signal_level)(driver)
        }
    }

    /// Enumerate tuning space name
    /// Returns None when index is out of range
    pub fn enum_tuning_space(&self, space_index: u32) -> Option<String> {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            let name_ptr = (vtable.enum_tuning_space)(driver, space_index);
            lpctstr_to_string(name_ptr)
        }
    }

    /// Enumerate channel name
    /// Returns None when index is out of range
    pub fn enum_channel_name(&self, space: u32, channel: u32) -> Option<String> {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            let name_ptr = (vtable.enum_channel_name)(driver, space, channel);
            lpctstr_to_string(name_ptr)
        }
    }

    /// Get all tuning spaces
    pub fn get_tuning_spaces(&self) -> Vec<(u32, String)> {
        let mut spaces = Vec::new();
        let mut index = 0;

        while let Some(name) = self.enum_tuning_space(index) {
            spaces.push((index, name));
            index += 1;
        }

        spaces
    }

    /// Get all channels in a tuning space
    pub fn get_channels(&self, space: u32) -> Vec<(u32, String)> {
        let mut channels = Vec::new();
        let mut index = 0;

        while let Some(name) = self.enum_channel_name(space, index) {
            channels.push((index, name));
            index += 1;
        }

        channels
    }

    /// Wait for TS stream data
    /// Returns the number of available packets or 0 on timeout
    pub fn wait_ts_stream(&self, timeout_ms: u32) -> u32 {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.wait_ts_stream)(driver, timeout_ms)
        }
    }

    /// Get count of ready TS packets
    pub fn get_ready_count(&self) -> u32 {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.get_ready_count)(driver)
        }
    }

    /// Get TS stream data (copy to buffer)
    ///
    /// Returns (bytes_read, bytes_remaining)
    pub fn get_ts_stream(&self, buffer: &mut [u8]) -> Result<(usize, usize)> {
        let driver = self.loader.driver_ptr();
        let mut size: u32 = buffer.len() as u32;
        let mut remain: u32 = 0;

        // Try GetTsStreamCopy first (more commonly implemented correctly)
        let result_copy = unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.get_ts_stream_copy)(driver, buffer.as_mut_ptr(), &mut size, &mut remain)
        };

        log::trace!(
            "GetTsStreamCopy: result={}, size={}, remain={}",
            result_copy,
            size,
            remain
        );

        if result_copy == TRUE && size > 0 {
            // Check if data looks like TS (first byte should be 0x47)
            if size >= 16 {
                log::trace!(
                    "GetTsStreamCopy data[0..16]: {:02X?}",
                    &buffer[..16]
                );
                // Check for TS sync byte
                if buffer[0] == 0x47 {
                    log::trace!("Valid TS data from GetTsStreamCopy");
                    return Ok((size as usize, remain as usize));
                }
                log::trace!("GetTsStreamCopy returned non-TS data, trying GetTsStreamPtr");
            } else {
                return Ok((size as usize, remain as usize));
            }
        }

        // Fallback to GetTsStreamPtr if GetTsStreamCopy fails or returns non-TS data
        let mut data_ptr: *mut u8 = std::ptr::null_mut();
        size = 0;
        remain = 0;

        let result_ptr = unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.get_ts_stream_ptr)(driver, &mut data_ptr, &mut size, &mut remain)
        };

        log::trace!(
            "GetTsStreamPtr: result={}, ptr_null={}, size={}, remain={}",
            result_ptr,
            data_ptr.is_null(),
            size,
            remain
        );

        if result_ptr == TRUE && !data_ptr.is_null() && size > 0 {
            let copy_size = std::cmp::min(size as usize, buffer.len());
            unsafe {
                std::ptr::copy_nonoverlapping(data_ptr, buffer.as_mut_ptr(), copy_size);
            }
            // Log first 16 bytes for debugging
            if copy_size >= 16 {
                log::trace!(
                    "GetTsStreamPtr data[0..16]: {:02X?}",
                    &buffer[..16]
                );
            }
            return Ok((copy_size, remain as usize));
        }

        // Neither method succeeded
        if result_copy == TRUE && size > 0 {
            // Return copy result even if it doesn't look like TS
            return Ok((size as usize, remain as usize));
        }

        Err(BonrecError::operation_error("Failed to get TS stream"))
    }

    /// Purge TS stream buffer
    pub fn purge_ts_stream(&self) {
        let driver = self.loader.driver_ptr();
        unsafe {
            let vtable = &*(*driver).vtable;
            (vtable.purge_ts_stream)(driver);
        }
    }

    /// Read TS stream with timeout
    ///
    /// Returns the TS data read within the timeout period
    pub fn read_ts_stream(&self, timeout_ms: u32) -> Result<Vec<u8>> {
        // Wait for data
        let wait_result = self.wait_ts_stream(timeout_ms);
        if wait_result == 0 {
            return Err(BonrecError::StreamTimeout { timeout_ms });
        }

        // Get ready count and allocate buffer
        let ready_count = self.get_ready_count();
        if ready_count == 0 {
            return Ok(Vec::new());
        }

        // Read all available data
        let mut all_data = Vec::new();
        let mut buffer = vec![0u8; DEFAULT_TS_BUFFER_SIZE];

        loop {
            let (bytes_read, remain) = self.get_ts_stream(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            all_data.extend_from_slice(&buffer[..bytes_read]);
            if remain == 0 {
                break;
            }
        }

        Ok(all_data)
    }
}

impl Drop for Tuner {
    fn drop(&mut self) {
        self.close();
    }
}

/// Guard for tuner open/close lifecycle
pub struct TunerGuard<'a> {
    tuner: &'a mut Tuner,
}

impl<'a> TunerGuard<'a> {
    /// Create a new tuner guard, opening the tuner
    pub fn new(tuner: &'a mut Tuner) -> Result<Self> {
        tuner.open()?;
        Ok(TunerGuard { tuner })
    }

    /// Get reference to the tuner
    pub fn tuner(&self) -> &Tuner {
        self.tuner
    }

    /// Get mutable reference to the tuner
    pub fn tuner_mut(&mut self) -> &mut Tuner {
        self.tuner
    }
}

impl<'a> Drop for TunerGuard<'a> {
    fn drop(&mut self) {
        self.tuner.close();
    }
}
