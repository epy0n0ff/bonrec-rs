//! BonDriver FFI type definitions
//!
//! This module defines the C/C++ interface types for BonDriver DLLs.

use std::ffi::c_void;

/// Windows BOOL type (32-bit integer)
pub type BOOL = i32;

/// Windows BYTE type (unsigned 8-bit)
pub type BYTE = u8;

/// Windows DWORD type (unsigned 32-bit)
pub type DWORD = u32;

/// Windows LPCTSTR type (pointer to wide char string)
pub type LPCTSTR = *const u16;

/// Windows TRUE value
pub const TRUE: BOOL = 1;

/// Windows FALSE value
pub const FALSE: BOOL = 0;

/// IBonDriver interface pointer
#[repr(C)]
pub struct IBonDriver {
    pub vtable: *const IBonDriverVtbl,
}

/// IBonDriver virtual method table
/// Note: GetTsStream has two overloads - the Ptr version comes before Copy in some implementations
#[repr(C)]
pub struct IBonDriverVtbl {
    // IBonDriver methods
    pub open_tuner: unsafe extern "system" fn(*mut IBonDriver) -> BOOL,
    pub close_tuner: unsafe extern "system" fn(*mut IBonDriver),
    pub set_channel_byte: unsafe extern "system" fn(*mut IBonDriver, BYTE) -> BOOL,
    pub get_signal_level: unsafe extern "system" fn(*mut IBonDriver) -> f32,
    pub wait_ts_stream: unsafe extern "system" fn(*mut IBonDriver, DWORD) -> DWORD,
    pub get_ready_count: unsafe extern "system" fn(*mut IBonDriver) -> DWORD,
    // Note: Ptr version comes first in PT series BonDriver vtable
    pub get_ts_stream_ptr:
        unsafe extern "system" fn(*mut IBonDriver, *mut *mut BYTE, *mut DWORD, *mut DWORD) -> BOOL,
    pub get_ts_stream_copy:
        unsafe extern "system" fn(*mut IBonDriver, *mut BYTE, *mut DWORD, *mut DWORD) -> BOOL,
    pub purge_ts_stream: unsafe extern "system" fn(*mut IBonDriver),
    pub release: unsafe extern "system" fn(*mut IBonDriver),
}

/// IBonDriver2 interface pointer (extends IBonDriver)
#[repr(C)]
pub struct IBonDriver2 {
    pub vtable: *const IBonDriver2Vtbl,
}

/// IBonDriver2 virtual method table (extends IBonDriverVtbl)
/// Note: GetTsStream has two overloads - the Ptr version comes before Copy in some implementations
#[repr(C)]
pub struct IBonDriver2Vtbl {
    // IBonDriver methods (inherited)
    pub open_tuner: unsafe extern "system" fn(*mut IBonDriver2) -> BOOL,
    pub close_tuner: unsafe extern "system" fn(*mut IBonDriver2),
    pub set_channel_byte: unsafe extern "system" fn(*mut IBonDriver2, BYTE) -> BOOL,
    pub get_signal_level: unsafe extern "system" fn(*mut IBonDriver2) -> f32,
    pub wait_ts_stream: unsafe extern "system" fn(*mut IBonDriver2, DWORD) -> DWORD,
    pub get_ready_count: unsafe extern "system" fn(*mut IBonDriver2) -> DWORD,
    // Note: Ptr version comes first in PT series BonDriver vtable
    pub get_ts_stream_ptr:
        unsafe extern "system" fn(*mut IBonDriver2, *mut *mut BYTE, *mut DWORD, *mut DWORD) -> BOOL,
    pub get_ts_stream_copy:
        unsafe extern "system" fn(*mut IBonDriver2, *mut BYTE, *mut DWORD, *mut DWORD) -> BOOL,
    pub purge_ts_stream: unsafe extern "system" fn(*mut IBonDriver2),
    pub release: unsafe extern "system" fn(*mut IBonDriver2),

    // IBonDriver2 extended methods
    pub get_tuner_name: unsafe extern "system" fn(*mut IBonDriver2) -> LPCTSTR,
    pub is_tuner_opening: unsafe extern "system" fn(*mut IBonDriver2) -> BOOL,
    pub enum_tuning_space: unsafe extern "system" fn(*mut IBonDriver2, DWORD) -> LPCTSTR,
    pub enum_channel_name: unsafe extern "system" fn(*mut IBonDriver2, DWORD, DWORD) -> LPCTSTR,
    pub set_channel: unsafe extern "system" fn(*mut IBonDriver2, DWORD, DWORD) -> BOOL,
    pub get_cur_space: unsafe extern "system" fn(*mut IBonDriver2) -> DWORD,
    pub get_cur_channel: unsafe extern "system" fn(*mut IBonDriver2) -> DWORD,
}

/// IBonDriver3 interface pointer (extends IBonDriver2, for satellite tuners)
#[repr(C)]
pub struct IBonDriver3 {
    pub vtable: *const IBonDriver3Vtbl,
}

/// IBonDriver3 virtual method table (extends IBonDriver2Vtbl)
/// Note: GetTsStream has two overloads - the Ptr version comes before Copy in some implementations
#[repr(C)]
pub struct IBonDriver3Vtbl {
    // IBonDriver methods (inherited)
    pub open_tuner: unsafe extern "system" fn(*mut IBonDriver3) -> BOOL,
    pub close_tuner: unsafe extern "system" fn(*mut IBonDriver3),
    pub set_channel_byte: unsafe extern "system" fn(*mut IBonDriver3, BYTE) -> BOOL,
    pub get_signal_level: unsafe extern "system" fn(*mut IBonDriver3) -> f32,
    pub wait_ts_stream: unsafe extern "system" fn(*mut IBonDriver3, DWORD) -> DWORD,
    pub get_ready_count: unsafe extern "system" fn(*mut IBonDriver3) -> DWORD,
    // Note: Ptr version comes first in PT series BonDriver vtable
    pub get_ts_stream_ptr:
        unsafe extern "system" fn(*mut IBonDriver3, *mut *mut BYTE, *mut DWORD, *mut DWORD) -> BOOL,
    pub get_ts_stream_copy:
        unsafe extern "system" fn(*mut IBonDriver3, *mut BYTE, *mut DWORD, *mut DWORD) -> BOOL,
    pub purge_ts_stream: unsafe extern "system" fn(*mut IBonDriver3),
    pub release: unsafe extern "system" fn(*mut IBonDriver3),

    // IBonDriver2 extended methods (inherited)
    pub get_tuner_name: unsafe extern "system" fn(*mut IBonDriver3) -> LPCTSTR,
    pub is_tuner_opening: unsafe extern "system" fn(*mut IBonDriver3) -> BOOL,
    pub enum_tuning_space: unsafe extern "system" fn(*mut IBonDriver3, DWORD) -> LPCTSTR,
    pub enum_channel_name: unsafe extern "system" fn(*mut IBonDriver3, DWORD, DWORD) -> LPCTSTR,
    pub set_channel: unsafe extern "system" fn(*mut IBonDriver3, DWORD, DWORD) -> BOOL,
    pub get_cur_space: unsafe extern "system" fn(*mut IBonDriver3) -> DWORD,
    pub get_cur_channel: unsafe extern "system" fn(*mut IBonDriver3) -> DWORD,

    // IBonDriver3 extended methods
    pub set_lnb_power: unsafe extern "system" fn(*mut IBonDriver3, BOOL) -> BOOL,
}

/// CreateBonDriver function type
pub type CreateBonDriverFn = unsafe extern "system" fn() -> *mut c_void;

/// Convert wide string pointer to Rust String
///
/// # Safety
/// The pointer must be a valid null-terminated wide string
pub unsafe fn lpctstr_to_string(ptr: LPCTSTR) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    // Find string length
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }

    if len == 0 {
        return Some(String::new());
    }

    // Convert to Rust string
    let slice = std::slice::from_raw_parts(ptr, len);
    Some(String::from_utf16_lossy(slice))
}
