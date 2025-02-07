#!/usr/bin/env python3
import os
import math
import argparse
from PIL import Image


def get_sorted_image_paths(root_dir):
    """
    Given a root directory, find all subfolders (sorted alphabetically),
    and within each folder, find all .webp and .jpg images (sorted alphabetically).
    Return a list of image file paths.
    """
    # Get a sorted list of subdirectories in the root_dir.
    subfolders = sorted(
        [
            os.path.join(root_dir, d)
            for d in os.listdir(root_dir)
            if os.path.isdir(os.path.join(root_dir, d))
        ]
    )

    image_paths = []
    # Process each folder in sorted order.
    for folder in subfolders:
        # Find .webp or .jpg files in the folder (sorted).
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
    Create a collage in which each image is placed in a square cell.
    Each image is resized (keeping its aspect ratio) to fit completely into a
    cell of dimensions cell_size x cell_size. The collage grid is as square as possible,
    with images placed left-to-right then top-to-bottom.
    """
    total_images = len(image_paths)
    if total_images == 0:
        print("No images found!")
        return

    # Decide grid dimensions: try to make a nearly square grid.
    ncols = math.ceil(math.sqrt(total_images))
    nrows = math.ceil(total_images / ncols)

    collage_width = ncols * cell_size
    collage_height = nrows * cell_size

    # Create a new blank image with a transparent background.
    collage = Image.new("RGBA", (collage_width, collage_height), (255, 255, 255, 0))

    for idx, img_path in enumerate(image_paths):
        try:
            with Image.open(img_path) as img:
                img = img.convert("RGBA")  # Ensure alpha channel if needed.
                orig_w, orig_h = img.size

                # Determine scale factor: scale so that the longer side equals cell_size.
                scale_factor = cell_size / max(orig_w, orig_h)
                new_w = int(orig_w * scale_factor)
                new_h = int(orig_h * scale_factor)

                # Resize the image (using high-quality downsampling).
                resized_img = img.resize((new_w, new_h), Image.LANCZOS)

                # Create a new cell image (square) with a transparent background.
                cell_img = Image.new("RGBA", (cell_size, cell_size), (255, 255, 255, 0))
                # Calculate position so that the image is centered in the cell.
                paste_x = (cell_size - new_w) // 2
                paste_y = (cell_size - new_h) // 2
                cell_img.paste(resized_img, (paste_x, paste_y), resized_img)

                # Compute position in the overall collage.
                row = idx // ncols
                col = idx % ncols
                pos_x = col * cell_size
                pos_y = row * cell_size
                collage.paste(cell_img, (pos_x, pos_y), cell_img)
        except Exception as e:
            print(f"Error processing '{img_path}': {e}")

    # Option 1: Convert to RGB to avoid issues with transparency
    collage.convert("RGB").save(output_path, "WEBP", lossless=True)
    # Option 2: If you want to keep transparency and are sure your Pillow supports it,
    # you can try saving directly:
    # collage.save(output_path, "WEBP", lossless=True)

    print(f"Collage saved to '{output_path}'")


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
