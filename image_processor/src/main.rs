use std::{ffi::CString, fs};

use clap::Parser;
use image::GenericImageView;
use image_processor::{args::Args, error::AppError, plugin::Plugin};

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    args.check_basic_paths_exists()?;

    let plugin_lib = Plugin::new(args.plugin_file()?)?;
    let interface = plugin_lib.interface()?;

    let img = image::open(&args.input)?;
    let (width, height) = img.dimensions();
    let mut rgba_data = img.to_rgba8();

    let params_content = fs::read_to_string(&args.params)?;
    let c_params = CString::new(params_content)?;

    let raw_data_ptr = rgba_data.as_mut_ptr();

    let error_code =
        unsafe { (interface.process_image_fn)(width, height, raw_data_ptr, c_params.as_ptr()) };

    if let Some(error) = AppError::from_plugin_error_code(error_code) {
        return Err(error.into());
    }

    rgba_data.save(&args.output)?;

    println!("Image saved successfully");

    Ok(())
}
