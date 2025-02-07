use clap::Parser;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use memmap2::MmapMut;
use std::cmp;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use tempfile::tempfile;

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

/// Recursively gathers image paths from subfolders (sorted by folder and filename).
fn get_sorted_image_paths(root_dir: &str) -> (Vec<PathBuf>, Vec<PathBuf>) {
    // List subdirectories (folders) in the root directory.
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

    // For each folder, collect image paths with .webp, .jpg, or .jpeg extension.
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

/// Creates the collage using a disk‑backed memory map to reduce in‑memory usage.
fn create_collage(image_paths: &[PathBuf], cell_size: u32, output_path: &str) -> image::ImageResult<()> {
    let total_images = image_paths.len() as u32;
    if total_images == 0 {
        eprintln!("No images found!");
        return Ok(());
    }
    // Calculate grid dimensions (nearly square).
    let ncols = (total_images as f64).sqrt().ceil() as u32;
    let nrows = (total_images + ncols - 1) / ncols; // ceiling division
    let collage_width = ncols * cell_size;
    let collage_height = nrows * cell_size;
    let num_pixels = (collage_width * collage_height) as usize;
    let buffer_size = num_pixels * 4; // 4 channels per pixel (RGBA)

    // Create a temporary file to back our memmap.
    let mut file = tempfile().expect("failed to create temp file");
    file.set_len(buffer_size as u64)
        .expect("failed to set file length");

    // Memory-map the file.
    let mut mmap = unsafe { MmapMut::map_mut(&file).expect("failed to map file") };

    // Initialize the memory to a “transparent white” background:
    // Set R, G, B to 255 and Alpha to 0 for every pixel.
    for i in 0..num_pixels {
        let offset = i * 4;
        mmap[offset] = 255;     // R
        mmap[offset + 1] = 255; // G
        mmap[offset + 2] = 255; // B
        mmap[offset + 3] = 0;   // A
    }

    // Process each image and paste it into its cell in the collage.
    for (idx, img_path) in image_paths.iter().enumerate() {
        // Attempt to open the image; if it fails, skip it.
        let img = match image::open(img_path) {
            Ok(im) => im,
            Err(e) => {
                eprintln!("Error processing {:?}: {}", img_path, e);
                // Use a 1x1 empty image as fallback.
                DynamicImage::new_rgba8(1, 1)
            }
        };

        let (orig_w, orig_h) = img.dimensions();
        // Compute scale factor so that the longer side equals the cell size.
        let scale_factor = cell_size as f32 / (cmp::max(orig_w, orig_h) as f32);
        let new_w = (orig_w as f32 * scale_factor).round() as u32;
        let new_h = (orig_h as f32 * scale_factor).round() as u32;
        let resized = img.resize(new_w, new_h, FilterType::Lanczos3).to_rgba8();

        // Determine which cell (column, row) the image should go in.
        let col = (idx as u32) % ncols;
        let row = (idx as u32) / ncols;
        let cell_x = col * cell_size;
        let cell_y = row * cell_size;
        // Center the resized image within its cell.
        let offset_x = cell_x + (cell_size - new_w) / 2;
        let offset_y = cell_y + (cell_size - new_h) / 2;

        // Copy pixels from the resized image into the correct region of the memmap.
        for y in 0..new_h {
            for x in 0..new_w {
                let pixel = resized.get_pixel(x, y);
                let target_x = offset_x + x;
                let target_y = offset_y + y;
                if target_x < collage_width && target_y < collage_height {
                    let index = ((target_y * collage_width + target_x) * 4) as usize;
                    mmap[index] = pixel[0];
                    mmap[index + 1] = pixel[1];
                    mmap[index + 2] = pixel[2];
                    mmap[index + 3] = pixel[3];
                }
            }
        }
    }
    mmap.flush().expect("failed to flush mmap");

    // At this point, the memmap contains the full collage.
    // Convert the memory-mapped data into a Vec<u8>.
    // (The final conversion requires an owned buffer.)
    let data = mmap.to_vec();
    let collage_buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(collage_width, collage_height, data)
        .expect("Failed to create ImageBuffer");

    // Save the final collage in WebP format.
    collage_buffer.save_with_format(output_path, image::ImageFormat::WebP)?;
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
