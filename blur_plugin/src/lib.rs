//! Image processor plugin for applying blur to image

#![deny(unreachable_pub)]
#![warn(missing_docs)]

use log::error;
use plugin_errors::PluginError;
use serde::Deserialize;
use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};
use std::panic::catch_unwind;

#[derive(Debug, Deserialize)]
struct BlurParams {
    radius: u32,
    iterations: u32,
    weighted: bool,
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

        let config: BlurParams = match serde_json::from_str(&params_str) {
            Ok(p) => p,
            Err(_) => return PluginError::InvalidParams as i32,
        };

        if config.radius == 0 || config.iterations == 0 {
            return PluginError::Ok as i32;
        }

        let Some(data_size) = (width as usize)
            .checked_mul(height as usize)
            .and_then(|res| res.checked_mul(4))
        else {
            return PluginError::SizeIsTooBig as i32;
        };

        // SAFETY: rgba_data must have at least data_size bytes
        let pixels = unsafe { std::slice::from_raw_parts_mut(rgba_data, data_size) };

        let mut buffer = vec![0u8; pixels.len()];

        for _ in 0..config.iterations {
            if config.weighted {
                apply_weighted_blur(
                    width as usize,
                    height as usize,
                    pixels,
                    &mut buffer,
                    config.radius as usize,
                );
            } else {
                apply_box_blur(
                    width as usize,
                    height as usize,
                    pixels,
                    &mut buffer,
                    config.radius as usize,
                );
            }

            pixels.copy_from_slice(&buffer);
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

fn apply_box_blur(width: usize, height: usize, src: &[u8], dst: &mut [u8], radius: usize) {
    for y in 0..height {
        for x in 0..width {
            let mut r_acc = 0u32;
            let mut g_acc = 0u32;
            let mut b_acc = 0u32;
            let mut count = 0u32;

            for ky in (y as isize - radius as isize)..=(y as isize + radius as isize) {
                for kx in (x as isize - radius as isize)..=(x as isize + radius as isize) {
                    if ky >= 0 && ky < height as isize && kx >= 0 && kx < width as isize {
                        let idx = (ky as usize * width + kx as usize) * 4;
                        r_acc += src[idx] as u32;
                        g_acc += src[idx + 1] as u32;
                        b_acc += src[idx + 2] as u32;
                        count += 1;
                    }
                }
            }

            let out_idx = (y * width + x) * 4;
            dst[out_idx] = (r_acc / count) as u8;
            dst[out_idx + 1] = (g_acc / count) as u8;
            dst[out_idx + 2] = (b_acc / count) as u8;
            dst[out_idx + 3] = src[out_idx + 3];
        }
    }
}

fn apply_weighted_blur(width: usize, height: usize, src: &[u8], dst: &mut [u8], radius: usize) {
    let radius_i = radius as isize;
    let sigma = (radius as f32) / 2.0;

    // generate weight kernel
    let size = radius * 2 + 1;
    let mut kernel = vec![0.0f32; size * size];
    let mut sum = 0.0f32;

    for ky in -radius_i..=radius_i {
        for kx in -radius_i..=radius_i {
            let dist_sq = (kx * kx + ky * ky) as f32;
            let weight = (-(dist_sq / (2.0 * sigma * sigma))).exp();
            kernel[((ky + radius_i) as usize * size) + (kx + radius_i) as usize] = weight;
            sum += weight;
        }
    }

    // normalize weights
    for w in kernel.iter_mut() {
        *w /= sum;
    }

    // apply weighted blur
    for y in 0..height {
        for x in 0..width {
            let mut r_acc = 0.0f32;
            let mut g_acc = 0.0f32;
            let mut b_acc = 0.0f32;

            for ky in -radius_i..=radius_i {
                for kx in -radius_i..=ky {
                    let py = (y as isize + ky).clamp(0, height as isize - 1) as usize;
                    let px = (x as isize + kx).clamp(0, width as isize - 1) as usize;

                    let weight =
                        kernel[((ky + radius_i) as usize * size) + (kx + radius_i) as usize];
                    let idx = (py * width + px) * 4;

                    r_acc += src[idx] as f32 * weight;
                    g_acc += src[idx + 1] as f32 * weight;
                    b_acc += src[idx + 2] as f32 * weight;
                }
            }

            let out_idx = (y * width + x) * 4;
            dst[out_idx] = r_acc.round() as u8;
            dst[out_idx + 1] = g_acc.round() as u8;
            dst[out_idx + 2] = b_acc.round() as u8;
            dst[out_idx + 3] = src[out_idx + 3];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // Helper function to create a dummy image buffer
    fn create_test_image(width: u32, height: u32, fill_color: u8) -> Vec<u8> {
        vec![fill_color; (width * height * 4) as usize]
    }

    #[test]
    fn test_process_image_null_rgba_data() {
        let params =
            CString::new(r#"{ "radius": 1, "iterations": 1, "weighted": false }"#).unwrap();
        let result = unsafe { process_image(1, 1, std::ptr::null_mut(), params.as_ptr()) };
        assert_eq!(result, PluginError::NullPointer as i32);
    }

    #[test]
    fn test_process_image_null_params() {
        let width = 1;
        let height = 1;
        let mut rgba_data = create_test_image(width, height, 0);
        let result =
            unsafe { process_image(width, height, rgba_data.as_mut_ptr(), std::ptr::null()) };
        assert_eq!(result, PluginError::NullPointer as i32);
    }

    #[test]
    fn test_process_image_invalid_json_params() {
        let width = 1;
        let height = 1;
        let mut rgba_data = create_test_image(width, height, 0);
        let params =
            CString::new(r#"{ "radius": 1, "iterations": 1, "weighted": false, }"#).unwrap(); // Trailing comma makes it invalid JSON
        let result =
            unsafe { process_image(width, height, rgba_data.as_mut_ptr(), params.as_ptr()) };
        assert_eq!(result, PluginError::InvalidParams as i32);
    }

    #[test]
    fn test_process_image_missing_fields_in_params() {
        let width = 1;
        let height = 1;
        let mut rgba_data = create_test_image(width, height, 0);
        let params = CString::new(r#"{ "radius": 1 }"#).unwrap(); // Missing iterations and weighted
        let result =
            unsafe { process_image(width, height, rgba_data.as_mut_ptr(), params.as_ptr()) };
        assert_eq!(result, PluginError::InvalidParams as i32);
    }

    #[test]
    fn test_size_too_big() {
        let mut rgba_data = vec![0u8; 4];
        let params =
            CString::new(r#"{ "radius": 1, "iterations": 1, "weighted": false }"#).unwrap();
        let result =
            unsafe { process_image(u32::MAX, u32::MAX, rgba_data.as_mut_ptr(), params.as_ptr()) };

        assert_eq!(result, PluginError::SizeIsTooBig as i32);
    }

    #[test]
    fn test_process_image_does_something_if_no_errors() {
        let width = 10;
        let height = 10;
        let mut rgba_data = create_test_image(width, height, 0);
        for i in 0..rgba_data.len() {
            rgba_data[i] = (i & 0xff) as u8;
        }
        let original_data = rgba_data.clone();
        let params =
            CString::new(r#"{ "radius": 1, "iterations": 1, "weighted": false }"#).unwrap();
        let result =
            unsafe { process_image(width, height, rgba_data.as_mut_ptr(), params.as_ptr()) };

        assert_eq!(result, PluginError::Ok as i32);
        assert_ne!(rgba_data, original_data);
    }
}
