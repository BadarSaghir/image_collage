// collage.go
package main

import (
	"flag"
	"fmt"
	"image"
	"image/color"
	"image/draw"
	"image/jpeg"
	_ "image/png" // in case you add png support later
	"log"
	"math"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/chai2010/webp"
	mmap "github.com/edsrzf/mmap-go"
	xdraw "golang.org/x/image/draw"
)

// loadImage opens the image file at path and decodes it.
// It supports .webp and .jpg (case‑insensitive).
func loadImage(path string) (image.Image, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	ext := strings.ToLower(filepath.Ext(path))
	switch ext {
	case ".webp":
		// Use the webp package to decode.
		return webp.Decode(f)
	case ".jpg", ".jpeg":
		return jpeg.Decode(f)
	default:
		return nil, fmt.Errorf("unsupported file extension: %s", ext)
	}
}

// getSortedImagePaths returns a slice of image file paths gathered from the sorted subfolders of rootDir.
// It also returns a slice of subfolder paths (in sorted order) for later per‑folder counting.
func getSortedImagePaths(rootDir string) ([]string, []string, error) {
	entries, err := os.ReadDir(rootDir)
	if err != nil {
		return nil, nil, err
	}

	var subfolders []string
	for _, e := range entries {
		if e.IsDir() {
			subfolders = append(subfolders, filepath.Join(rootDir, e.Name()))
		}
	}
	sort.Strings(subfolders)

	var imagePaths []string
	for _, folder := range subfolders {
		files, err := os.ReadDir(folder)
		if err != nil {
			log.Printf("Warning: could not read folder %s: %v", folder, err)
			continue
		}

		var imgsInFolder []string
		for _, file := range files {
			if file.IsDir() {
				continue
			}
			lowName := strings.ToLower(file.Name())
			if strings.HasSuffix(lowName, ".webp") ||
				strings.HasSuffix(lowName, ".jpg") ||
				strings.HasSuffix(lowName, ".jpeg") {
				imgsInFolder = append(imgsInFolder, filepath.Join(folder, file.Name()))
			}
		}
		sort.Strings(imgsInFolder)
		imagePaths = append(imagePaths, imgsInFolder...)
	}
	return imagePaths, subfolders, nil
}

// createCollage creates the collage image given the list of image paths, cell size, and writes the result to outputPath.
// This version uses a disk‑backed memory map to hold the collage buffer.
func createCollage(imagePaths []string, cellSize int, outputPath string) error {
	totalImages := len(imagePaths)
	if totalImages == 0 {
		return fmt.Errorf("no images found")
	}

	// Calculate grid dimensions (nearly square).
	ncols := int(math.Ceil(math.Sqrt(float64(totalImages))))
	nrows := int(math.Ceil(float64(totalImages) / float64(ncols)))
	collageWidth := ncols * cellSize
	collageHeight := nrows * cellSize
	bufferSize := collageWidth * collageHeight * 4 // 4 bytes per pixel (RGBA)

	// Create a temporary file to back our collage buffer.
	tmpFile, err := os.CreateTemp("", "collage-*.tmp")
	if err != nil {
		return fmt.Errorf("failed to create temp file: %v", err)
	}
	// Ensure the file is removed after we're done.
	defer os.Remove(tmpFile.Name())

	// Set the file size.
	if err := tmpFile.Truncate(int64(bufferSize)); err != nil {
		return fmt.Errorf("failed to truncate temp file: %v", err)
	}

	// Memory-map the temporary file (read-write).
	mapped, err := mmap.Map(tmpFile, mmap.RDWR, 0)
	if err != nil {
		return fmt.Errorf("failed to memory-map file: %v", err)
	}
	// Ensure the mapping is unmapped later.
	defer mapped.Unmap()

	// Create an RGBA image that uses the memory-mapped slice as its pixel buffer.
	collage := &image.RGBA{
		Pix:    mapped,
		Stride: collageWidth * 4,
		Rect:   image.Rect(0, 0, collageWidth, collageHeight),
	}

	// Fill the collage background with transparent white (R, G, B = 255, Alpha = 0).
	draw.Draw(collage, collage.Rect, &image.Uniform{color.RGBA{255, 255, 255, 0}}, image.Point{}, draw.Src)

	// Process each image.
	for idx, imgPath := range imagePaths {
		img, err := loadImage(imgPath)
		if err != nil {
			log.Printf("Error processing '%s': %v", imgPath, err)
			continue
		}

		// Convert to RGBA if needed.
		bounds := img.Bounds()
		origW, origH := bounds.Dx(), bounds.Dy()

		// Determine scale factor (so that the longer side equals cellSize).
		scaleFactor := float64(cellSize) / float64(max(origW, origH))
		newW := int(float64(origW) * scaleFactor)
		newH := int(float64(origH) * scaleFactor)

		// Create a new RGBA image for the resized image.
		resized := image.NewRGBA(image.Rect(0, 0, newW, newH))
		// Use high-quality scaling.
		xdraw.CatmullRom.Scale(resized, resized.Rect, img, bounds, xdraw.Over, nil)

		// Compute cell position.
		row := idx / ncols
		col := idx % ncols
		cellX := col * cellSize
		cellY := row * cellSize
		// Center the resized image in the cell.
		offsetX := cellX + (cellSize-newW)/2
		offsetY := cellY + (cellSize-newH)/2

		// Paste the resized image onto the collage.
		destRect := image.Rect(offsetX, offsetY, offsetX+newW, offsetY+newH)
		draw.Draw(collage, destRect, resized, image.Point{}, draw.Over)
	}

	// Ensure any changes to the memory map are flushed.
	if err := mapped.Flush(); err != nil {
		return fmt.Errorf("failed to flush memory map: %v", err)
	}

	// Save the final collage as a WebP image.
	outFile, err := os.Create(outputPath)
	if err != nil {
		return fmt.Errorf("failed to create output file: %v", err)
	}
	defer outFile.Close()

	// Use lossless encoding.
	options := &webp.Options{Lossless: true}
	if err := webp.Encode(outFile, collage, options); err != nil {
		return fmt.Errorf("failed to encode WebP: %v", err)
	}
	fmt.Printf("Collage saved to '%s'\n", outputPath)
	return nil
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func main() {
	// Parse command-line arguments.
	inputDir := flag.String("input_dir", "", "Path to the root directory containing subfolders with images")
	outputFile := flag.String("output_file", "", "Output collage file (e.g. collage.webp)")
	cellSize := flag.Int("cell_size", 200, "Size in pixels for each cell (default: 200)")
	flag.Parse()

	if *inputDir == "" || *outputFile == "" {
		flag.Usage()
		os.Exit(1)
	}

	// Get sorted image paths.
	imagePaths, subfolders, err := getSortedImagePaths(*inputDir)
	if err != nil {
		log.Fatalf("Error: %v\n", err)
	}

	// Count images per subfolder.
	totalCount := 0
	fmt.Println("Image counts per folder:")
	for _, folder := range subfolders {
		files, err := os.ReadDir(folder)
		if err != nil {
			log.Printf("Warning: could not read folder %s: %v", folder, err)
			continue
		}
		count := 0
		for _, file := range files {
			lowName := strings.ToLower(file.Name())
			if !file.IsDir() && (strings.HasSuffix(lowName, ".webp") || strings.HasSuffix(lowName, ".jpg") || strings.HasSuffix(lowName, ".jpeg")) {
				count++
			}
		}
		totalCount += count
		fmt.Printf("  %s: %d images\n", folder, count)
	}
	fmt.Printf("\nTotal images found: %d\n", totalCount)

	if totalCount == 0 {
		log.Fatalf("No .webp or .jpg images found in the provided folders.")
	}

	// Create the collage.
	if err := createCollage(imagePaths, *cellSize, *outputFile); err != nil {
		log.Fatalf("Error creating collage: %v", err)
	}
}
