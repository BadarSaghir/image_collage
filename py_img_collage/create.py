#!/usr/bin/env python3
import os
import math
import argparse
import tempfile
import numpy as np
from PIL import Image

def get_sorted_image_paths(root_dir):
    """
    Given a root directory, find all subfolders (sorted alphabetically),
    and within each folder, find all .webp and .jpg images (sorted alphabetically).
    Return a list of image file paths and the list of subfolder paths.
    """
    subfolders = sorted(
        [
            os.path.join(root_dir, d)
            for d in os.listdir(root_dir)
            if os.path.isdir(os.path.join(root_dir, d))
        ]
    )

    image_paths = []
    for folder in subfolders:
        images = sorted(
            [
                os.path.join(folder, f)
                for f in os.listdir(folder)
                if f.lower().endswith(".webp") or f.lower().endswith(".jpg")
            ]
        )
        image_paths.extend(images)
    return image_paths, subfolders

def create_collage(image_paths, cell_size, output_path):
    """
    Create a collage where each image is resized (keeping its aspect ratio)
    to fit inside a cell of size (cell_size x cell_size). The images are arranged
    in a grid (nearly square) using a disk‑backed (memmap) array so that we don’t
    hold a huge collage entirely in RAM during processing.
    """
    total_images = len(image_paths)
    if total_images == 0:
        print("No images found!")
        return

    # Compute grid dimensions.
    ncols = math.ceil(math.sqrt(total_images))
    nrows = math.ceil(total_images / ncols)
    collage_width = ncols * cell_size
    collage_height = nrows * cell_size

    # Create a disk-backed NumPy memmap array for the collage.
    # We use 4 channels (RGBA) with dtype=uint8.
    temp_file = tempfile.NamedTemporaryFile(delete=False)
    temp_file.close()  # We only need the filename.
    collage_array = np.memmap(temp_file.name,
                              dtype=np.uint8,
                              mode='w+',
                              shape=(collage_height, collage_width, 4))
    # Initialize background to transparent white: (255, 255, 255, 0).
    collage_array[:, :, 0:3] = 255
    collage_array[:, :, 3] = 0

    # Process each image and “paste” it into the appropriate cell.
    for idx, img_path in enumerate(image_paths):
        try:
            with Image.open(img_path) as img:
                # Ensure we work in RGBA.
                img = img.convert("RGBA")
                orig_w, orig_h = img.size

                # Determine the scale factor so that the longer side equals cell_size.
                scale_factor = cell_size / max(orig_w, orig_h)
                new_w = int(orig_w * scale_factor)
                new_h = int(orig_h * scale_factor)

                # Resize the image with high-quality downsampling.
                resized_img = img.resize((new_w, new_h), Image.LANCZOS)

                # Create a temporary cell tile as a NumPy array.
                # Start with a transparent white background.
                cell_tile = np.zeros((cell_size, cell_size, 4), dtype=np.uint8)
                cell_tile[:, :, 0:3] = 255
                cell_tile[:, :, 3] = 0

                # Convert the resized image to a NumPy array.
                resized_array = np.array(resized_img)

                # Compute offsets to center the resized image in the cell.
                paste_x = (cell_size - new_w) // 2
                paste_y = (cell_size - new_h) // 2

                # Paste the resized image into the cell tile.
                cell_tile[paste_y:paste_y+new_h, paste_x:paste_x+new_w, :] = resized_array

                # Compute the cell's position in the collage.
                row = idx // ncols
                col = idx % ncols
                pos_x = col * cell_size
                pos_y = row * cell_size

                # Paste the cell tile into the collage memmap.
                collage_array[pos_y:pos_y+cell_size, pos_x:pos_x+cell_size, :] = cell_tile
        except Exception as e:
            print(f"Error processing '{img_path}': {e}")

    # Flush changes to disk.
    collage_array.flush()

    # --- Final conversion using frombuffer ---
    # Use frombuffer to create a PIL Image that uses the memmap's memory directly.
    # This avoids an extra full-array copy when converting the collage.
    try:
        collage_image = Image.frombuffer(
            "RGBA",
            (collage_width, collage_height),
            collage_array,
            "raw",
            "RGBA",
            0,
            1
        )
    except Exception as e:
        print("Error creating image from buffer:", e)
        # Fallback: create a full copy (this may use more memory)
        collage_image = Image.fromarray(np.array(collage_array), "RGBA")

    # Convert to RGB (if you don't need transparency or want to avoid WebP RGBA issues)
    collage_image.convert("RGB").save(output_path, "WEBP", lossless=True)
    print(f"Collage saved to '{output_path}'")

    # Clean up the temporary memmap file.
    os.remove(temp_file.name)

def main():
    parser = argparse.ArgumentParser(
        description="Create a collage where each cell is one image (no cropping) arranged in a grid. "
                    "The images are taken from sorted folders (first folder's images appear first, etc.)."
    )
    parser.add_argument(
        "input_dir", help="Path to the root directory containing subfolders with images"
    )
    parser.add_argument("output_file", help="Output collage file (e.g. collage.webp)")
    parser.add_argument(
        "--cell_size",
        type=int,
        default=200,
        help="Size in pixels for each cell (default: 200)",
    )
    args = parser.parse_args()

    # Get sorted image paths and list of subfolders.
    image_paths, subfolders = get_sorted_image_paths(args.input_dir)

    # Count and print images per subfolder.
    total_count = 0
    print("Image counts per folder:")
    for folder in subfolders:
        images = sorted(
            [
                f
                for f in os.listdir(folder)
                if f.lower().endswith(".webp") or f.lower().endswith(".jpg")
            ]
        )
        folder_count = len(images)
        total_count += folder_count
        print(f"  {folder}: {folder_count} images")

    # Print total images found.
    print(f"\nTotal images found: {total_count}")

    if total_count == 0:
        print("No .webp or .jpg images found in the provided folders.")
        return

    # Create the collage.
    create_collage(image_paths, args.cell_size, args.output_file)

if __name__ == "__main__":
    main()
