//! Shared plugin errors related library
#![deny(unreachable_pub)]
#![warn(missing_docs)]

/// Known plugin errors with mappings into i32 for ABI interaction
/// Used as return code form process_image function
#[repr(i32)]
pub enum PluginError {
    /// No error
    Ok = 0,

    /// Plugin unable to read parameters
    InvalidParams = 1,

    /// Null pointer is given to plugin
    NullPointer = 2,
}

impl PluginError {
    /// Map error code to PluginError if code is known
    pub fn from(code: i32) -> Option<PluginError> {
        match code {
            0 => Some(PluginError::Ok),
            1 => Some(PluginError::InvalidParams),
            2 => Some(PluginError::NullPointer),
            _ => None,
        }
    }
}
