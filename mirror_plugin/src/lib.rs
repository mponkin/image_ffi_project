//! Image processor plugin for mirroring image

#![deny(unreachable_pub)]
#![warn(missing_docs)]

use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};

use plugin_errors::PluginError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MirrorParams {
    horizontal: bool,
    vertical: bool,
}

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
#[unsafe(no_mangle)]
pub extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut c_uchar,
    params: *const c_char,
) -> i32 {
    // Prevent usage of null pointers
    if rgba_data.is_null() || params.is_null() {
        return PluginError::NullPointer as i32;
    }

    // SAFETY: `params` should point to a valid UTF-8 string ending with nul-terminator
    let c_str = unsafe { CStr::from_ptr(params) };
    let params_str = c_str.to_string_lossy();

    let config: MirrorParams = match serde_json::from_str(&params_str) {
        Ok(p) => p,
        Err(_) => return PluginError::InvalidParams as i32,
    };

    let data_size = (width * height * 4) as usize;

    // SAFETY: rgba_data must have at least data_size bytes
    let pixels = unsafe { std::slice::from_raw_parts_mut(rgba_data, data_size) };

    if config.horizontal {
        mirror_horizontal(width, height, pixels);
    }
    if config.vertical {
        mirror_vertical(width, height, pixels);
    }

    PluginError::Ok as i32
}

fn mirror_horizontal(width: u32, height: u32, pixels: &mut [u8]) {
    let width = width as usize;
    for y in 0..height as usize {
        let row_start = y * width * 4;
        let row_end = row_start + width * 4;
        let row = &mut pixels[row_start..row_end];

        for x in 0..(width / 2) {
            let left_idx = x * 4;
            let right_idx = (width - 1 - x) * 4;

            for i in 0..4 {
                row.swap(left_idx + i, right_idx + i);
            }
        }
    }
}

fn mirror_vertical(width: u32, height: u32, pixels: &mut [u8]) {
    let width = width as usize;
    let height = height as usize;
    let row_size = width * 4;

    for y in 0..(height / 2) {
        let top_row_idx = y * row_size;
        let bottom_row_idx = (height - 1 - y) * row_size;

        for i in 0..row_size {
            pixels.swap(top_row_idx + i, bottom_row_idx + i);
        }
    }
}
