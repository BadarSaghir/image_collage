use clap::Parser;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, Rgba};
use std::cmp;
use std::fs;
use std::path::{Path, PathBuf};

/// Create a collage from images in sorted subfolders.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to the root directory containing subfolders with images.
    input_dir: String,

    /// Output collage file (e.g. collage.webp).
    output_file: String,

    /// Size in pixels for each cell (default: 200).
    #[arg(long, default_value_t = 200)]
    cell_size: u32,
}

fn get_sorted_image_paths(root_dir: &str) -> (Vec<PathBuf>, Vec<PathBuf>) {
    // List subdirectories in the root directory (sorted).
    let mut subfolders = fs::read_dir(root_dir)
        .expect("Unable to read input directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.path().is_dir() {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    subfolders.sort();

    let mut image_paths = Vec::new();
    for folder in &subfolders {
        let mut imgs_in_folder = fs::read_dir(folder)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                if entry.path().is_file() {
                    let ext = entry
                        .path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if ext == "webp" || ext == "jpg" || ext == "jpeg" {
                        Some(entry.path())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        imgs_in_folder.sort();
        image_paths.extend(imgs_in_folder);
    }
    (image_paths, subfolders)
}

fn create_collage(image_paths: &[PathBuf], cell_size: u32, output_path: &str) -> image::ImageResult<()> {
    let total_images = image_paths.len() as u32;
    if total_images == 0 {
        eprintln!("No images found!");
        return Ok(());
    }
    // Calculate grid dimensions.
    let ncols = (total_images as f64).sqrt().ceil() as u32;
    let nrows = (total_images + ncols - 1) / ncols; // ceiling division
    let collage_width = ncols * cell_size;
    let collage_height = nrows * cell_size;

    // Create a new image with transparent background.
    let mut collage: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(collage_width, collage_height, Rgba([0, 0, 0, 0]));

    for (idx, img_path) in image_paths.iter().enumerate() {
        let img = image::open(img_path).unwrap_or_else(|e| {
            eprintln!("Error processing {:?}: {}", img_path, e);
            // Return an empty image if error occurs.
            DynamicImage::new_rgba8(1, 1)
        });
        let (orig_w, orig_h) = img.dimensions();
        // Compute scale factor so that the longer side equals cell_size.
        let scale_factor = cell_size as f32 / (cmp::max(orig_w, orig_h) as f32);
        let new_w = (orig_w as f32 * scale_factor).round() as u32;
        let new_h = (orig_h as f32 * scale_factor).round() as u32;
        // Resize the image using a high-quality filter.
        let resized = img.resize(new_w, new_h, FilterType::Lanczos3);

        // Determine the cell position.
        let col = (idx as u32) % ncols;
        let row = (idx as u32) / ncols;
        let cell_x = col * cell_size;
        let cell_y = row * cell_size;
        // Center the resized image in the cell.
        let offset_x = cell_x + (cell_size - new_w) / 2;
        let offset_y = cell_y + (cell_size - new_h) / 2;

        // Paste the resized image onto the collage.
        collage.copy_from(&resized.to_rgba8(), offset_x, offset_y).unwrap();
    }

    // Save the final collage in WebP format.
    // The image crate supports saving in WebP if the format is detected from the extension.
    collage.save_with_format(output_path, image::ImageFormat::WebP)?;
    println!("Collage saved to '{}'", output_path);
    Ok(())
}

fn main() {
    let args = Args::parse();

    let (image_paths, subfolders) = get_sorted_image_paths(&args.input_dir);

    // Count and print images per subfolder.
    let mut total_count = 0;
    println!("Image counts per folder:");
    for folder in subfolders {
        let count = fs::read_dir(&folder)
            .unwrap()
            .filter(|entry| {
                if let Ok(entry) = entry {
                    if entry.path().is_file() {
                        let ext = entry
                            .path()
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        return ext == "webp" || ext == "jpg" || ext == "jpeg";
                    }
                }
                false
            })
            .count();
        total_count += count;
        println!("  {:?}: {} images", folder, count);
    }
    println!("\nTotal images found: {}", total_count);

    if total_count == 0 {
        eprintln!("No .webp or .jpg images found in the provided folders.");
        return;
    }

    if let Err(e) = create_collage(&image_paths, args.cell_size, &args.output_file) {
        eprintln!("Error creating collage: {}", e);
    }
}
