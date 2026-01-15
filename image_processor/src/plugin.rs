//! Plugin initialization and interface
use std::{
    os::raw::{c_char, c_uchar},
    path::PathBuf,
};

use libloading::{Library, Symbol};

/// Struct contatining plugin library
pub struct Plugin {
    plugin: Library,
}

/// Struct to hold pointer for image process function from plugin
pub struct PluginInterface<'a> {
    /// Image conversion function. Runs in-place
    ///
    /// # Arguments
    ///
    /// * `width` - image width in pixels
    /// * `height` - image height in pixels
    /// * `rgba_data` - pointer to image data. Image conversion runs in place so it will contain result data in case of successful conversion
    /// * `params` - pointer to params string
    ///
    /// # Safety
    ///
    /// Pointers are checked for being non-null before usage
    /// `params` should point to a valid UTF-8 string ending with nul-terminator
    /// `rgba_data` must have at least data_size bytes
    ///
    pub process_image_fn: Symbol<
        'a,
        unsafe extern "C" fn(
            width: u32,
            height: u32,
            rgba_data: *mut c_uchar,
            params: *const c_char,
        ) -> i32,
    >,
}

impl Plugin {
    /// Find and load a dynamic library
    ///
    /// `plugin_file` should point to existing dynamic library
    ///
    /// Safety: it is expected for plugin to export `process_image` function,
    /// not trying to complete any harmful operations and not use any pointers after image conversion is finished
    pub fn new(plugin_file: PathBuf) -> Result<Self, libloading::Error> {
        Ok(Plugin {
            plugin: unsafe { Library::new(plugin_file) }?,
        })
    }

    /// Gets a pointer to PluginInterface struct
    ///
    /// Safety: it is expected for plugin to export `process_image` function,
    /// not trying to complete any harmful operations and not use any pointers after image conversion is finished
    pub fn interface(&self) -> Result<PluginInterface<'_>, libloading::Error> {
        Ok(PluginInterface {
            process_image_fn: unsafe { self.plugin.get("process_image") }?,
        })
    }
}
