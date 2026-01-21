//! App errors list and logic
use plugin_errors::PluginError;
use thiserror::Error;

/// Checked app errors
#[derive(Debug, Error)]
pub enum AppError {
    /// Input file not found
    #[error("Input file '{0}' not found")]
    InputFileNotFound(String),

    /// Params file not found
    #[error("Params file '{0}' not found")]
    ParamsFileNotFound(String),

    /// Plugin directory not found
    #[error("Plugin directory '{0}' not found")]
    PluginDirectoryNotFound(String),

    /// Plugin not found in directory
    #[error("Plugin '{0}' not found")]
    PluginNotFound(String),

    /// Null pointer is passed to plugin for image data or parameters string
    #[error("Plugin received null pointer")]
    NullPointer,

    /// Unable to parse plugin parameters
    #[error("Plugin parameters are incorrect")]
    PluginInvalidParams,

    /// Plugin finished work with error and returned unexpected error code
    #[error("Plugin returned unknown error code {0}")]
    PluginUnknownErrorCode(i32),

    /// Panic happened during image processing
    #[error("Panic happened during image processing")]
    PluginPanic,
}

impl AppError {
    /// Convert plugin return code to Some(AppError) or None if plugin finished without error
    pub fn from_plugin_error_code(code: i32) -> Option<Self> {
        let plugin_error = PluginError::from(code);

        match plugin_error {
            Some(PluginError::Ok) => None,
            Some(PluginError::InvalidParams) => Some(AppError::PluginInvalidParams),
            Some(PluginError::NullPointer) => Some(AppError::NullPointer),
            Some(PluginError::Panic) => Some(AppError::PluginPanic),
            None => Some(AppError::PluginUnknownErrorCode(code)),
        }
    }
}
