//! Image processor plugin for applying blur to image

#![deny(unreachable_pub)]
#![warn(missing_docs)]

use plugin_errors::PluginError;
use serde::Deserialize;
use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};

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

    let data_size = (width * height * 4) as usize;

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
                for kx in -radius_i..=radius_i {
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
