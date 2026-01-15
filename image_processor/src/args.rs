//! CLI arguments of app
use std::path::PathBuf;

use clap::Parser;

use crate::error::AppError;

/// CLI arguments struct
#[derive(Parser, Debug)]
#[command(about = "Image Converter with Plugin System")]
pub struct Args {
    /// Path to input image
    #[arg(long, value_name = "FILE")]
    pub input: PathBuf,

    /// Path to save result
    #[arg(long, value_name = "FILE")]
    pub output: PathBuf,

    /// Name of image conversion plugin
    #[arg(long, value_name = "PLUGIN_NAME")]
    pub plugin: String,

    /// Path to file with params of conversion plugin
    #[arg(long, value_name = "FILE")]
    pub params: PathBuf,

    /// Path to plugins directory
    #[arg(long, default_value = "target/debug", value_name = "DIR")]
    pub plugin_path: PathBuf,
}

impl Args {
    /// Verify all required files and directories exist
    /// return AppError if something does not exist
    pub fn check_basic_paths_exists(&self) -> Result<(), AppError> {
        if !self.input.exists() {
            return Err(AppError::InputFileNotFound(
                self.input.to_string_lossy().to_string(),
            ));
        }

        if !self.params.exists() {
            return Err(AppError::ParamsFileNotFound(
                self.params.to_string_lossy().to_string(),
            ));
        }

        if !self.plugin_path.exists() {
            return Err(AppError::PluginDirectoryNotFound(
                self.plugin_path.to_string_lossy().to_string(),
            ));
        }

        Ok(())
    }

    /// Verify that plugin exists in plugins directory and return `PathBuf` to it or `AppError` otherwise
    pub fn plugin_file(&self) -> Result<PathBuf, AppError> {
        let plugin_filename = libloading::library_filename(&self.plugin);
        let plugin_file = self.plugin_path.join(plugin_filename);

        if !plugin_file.exists() {
            return Err(AppError::PluginNotFound(
                plugin_file.to_string_lossy().to_string(),
            ));
        }

        Ok(plugin_file)
    }
}
