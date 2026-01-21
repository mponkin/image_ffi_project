//! Image processor plugin for mirroring image

#![deny(unreachable_pub)]
#![warn(missing_docs)]

use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};
use std::panic::catch_unwind;

use log::error;
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
pub unsafe extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut c_uchar,
    params: *const c_char,
) -> i32 {
    let result = catch_unwind(move || {
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

        let Some(data_size) = (width as usize)
            .checked_mul(height as usize)
            .and_then(|res| res.checked_mul(4))
        else {
            return PluginError::SizeIsTooBig as i32;
        };

        // SAFETY: rgba_data must have at least data_size bytes
        let pixels = unsafe { std::slice::from_raw_parts_mut(rgba_data, data_size) };

        if config.horizontal {
            mirror_horizontal(width, height, pixels);
        }
        if config.vertical {
            mirror_vertical(width, height, pixels);
        }

        PluginError::Ok as i32
    });

    match result {
        Ok(status) => status,
        Err(e) => {
            error!("panic in process_image {e:?}");
            PluginError::Panic as i32
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ffi::CString, u32};

    fn create_test_image(width: u32, height: u32) -> Vec<u8> {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.push(x as u8);
                pixels.push(y as u8);
                pixels.push(0);
                pixels.push(255);
            }
        }
        pixels
    }

    #[test]
    fn test_process_image_null_rgba_data() {
        let params = CString::new(r#"{ "horizontal": true, "vertical": false }"#).unwrap();
        let result = unsafe { process_image(1, 1, std::ptr::null_mut(), params.as_ptr()) };
        assert_eq!(result, PluginError::NullPointer as i32);
    }

    #[test]
    fn test_process_image_null_params() {
        let width = 1;
        let height = 1;
        let mut rgba_data = create_test_image(width, height);
        let result =
            unsafe { process_image(width, height, rgba_data.as_mut_ptr(), std::ptr::null()) };
        assert_eq!(result, PluginError::NullPointer as i32);
    }

    #[test]
    fn test_process_image_invalid_json_params() {
        let width = 1;
        let height = 1;
        let mut rgba_data = create_test_image(width, height);
        let params = CString::new(r#"{ "horizontal": true, "vertical": false, }"#).unwrap(); // Trailing comma
        let result =
            unsafe { process_image(width, height, rgba_data.as_mut_ptr(), params.as_ptr()) };
        assert_eq!(result, PluginError::InvalidParams as i32);
    }

    #[test]
    fn test_process_image_missing_fields_in_params() {
        let width = 10;
        let height = 10;
        let mut rgba_data = create_test_image(width, height);
        let params = CString::new(r#"{ "horizontal": true }"#).unwrap(); // Missing vertical
        let result =
            unsafe { process_image(width, height, rgba_data.as_mut_ptr(), params.as_ptr()) };
        assert_eq!(result, PluginError::InvalidParams as i32);
    }

    #[test]
    fn test_size_too_big() {
        let mut rgba_data = vec![0u8; 4];
        let params = CString::new(r#"{ "horizontal": true, "vertical": true }"#).unwrap();
        let result =
            unsafe { process_image(u32::MAX, u32::MAX, rgba_data.as_mut_ptr(), params.as_ptr()) };

        assert_eq!(result, PluginError::SizeIsTooBig as i32);
    }

    #[test]
    fn test_process_image_does_something_if_no_errors() {
        let width = 2;
        let height = 2;
        let mut rgba_data = create_test_image(width, height);
        let original_data = rgba_data.clone();
        let params = CString::new(r#"{ "horizontal": true, "vertical": true }"#).unwrap();
        let result =
            unsafe { process_image(width, height, rgba_data.as_mut_ptr(), params.as_ptr()) };

        assert_eq!(result, PluginError::Ok as i32);
        assert_ne!(rgba_data, original_data)
    }

    #[test]
    fn test_mirror_horizontal_2x2() {
        let width = 2;
        let height = 2;

        let mut pixels = create_test_image(width, height);

        let expected = vec![1, 0, 0, 255, 0, 0, 0, 255, 1, 1, 0, 255, 0, 1, 0, 255];
        mirror_horizontal(width, height, &mut pixels);
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_mirror_horizontal_3x1() {
        let width = 3;
        let height = 1;

        let mut pixels = create_test_image(width, height);

        let expected = vec![2, 0, 0, 255, 1, 0, 0, 255, 0, 0, 0, 255];
        mirror_horizontal(width, height, &mut pixels);
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_mirror_horizontal_single_pixel() {
        let width = 1;
        let height = 1;
        let mut pixels = create_test_image(width, height);
        let original_pixels = pixels.clone();
        mirror_horizontal(width, height, &mut pixels);
        assert_eq!(pixels, original_pixels);
    }

    #[test]
    fn test_mirror_vertical_2x2() {
        let width = 2;
        let height = 2;

        let mut pixels = create_test_image(width, height);

        let expected = vec![0, 1, 0, 255, 1, 1, 0, 255, 0, 0, 0, 255, 1, 0, 0, 255];
        mirror_vertical(width, height, &mut pixels);
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_mirror_vertical_1x3() {
        let width = 1;
        let height = 3;

        let mut pixels = create_test_image(width, height);

        let expected = vec![0, 2, 0, 255, 0, 1, 0, 255, 0, 0, 0, 255];
        mirror_vertical(width, height, &mut pixels);
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_mirror_vertical_single_pixel() {
        let width = 1;
        let height = 1;
        let mut pixels = create_test_image(width, height);
        let original_pixels = pixels.clone();
        mirror_vertical(width, height, &mut pixels);
        assert_eq!(pixels, original_pixels);
    }
}
