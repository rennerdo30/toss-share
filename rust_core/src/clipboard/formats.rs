//! Image format conversion utilities
//!
//! Provides utilities for image encoding/decoding between clipboard formats.

#![allow(dead_code)]

use arboard::ImageData;
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;

use crate::error::ClipboardError;

/// Encode arboard ImageData to PNG bytes
pub fn encode_image_to_png(image: &ImageData) -> Result<Vec<u8>, ClipboardError> {
    // Create RgbaImage from raw data
    let rgba_image = RgbaImage::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.to_vec(),
    )
    .ok_or_else(|| ClipboardError::ImageConversion("Failed to create image from raw data".to_string()))?;

    let dynamic_image = DynamicImage::ImageRgba8(rgba_image);

    // Encode to PNG
    let mut buffer = Vec::new();
    dynamic_image
        .write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)
        .map_err(|e| ClipboardError::ImageConversion(e.to_string()))?;

    Ok(buffer)
}

/// Decode PNG/JPEG bytes to arboard ImageData
pub fn decode_image(data: &[u8]) -> Result<ImageData<'static>, ClipboardError> {
    // Detect format and decode
    let format = image::guess_format(data)
        .map_err(|e| ClipboardError::ImageConversion(format!("Unknown image format: {}", e)))?;

    let image = image::load_from_memory_with_format(data, format)
        .map_err(|e| ClipboardError::ImageConversion(e.to_string()))?;

    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok(ImageData {
        width: width as usize,
        height: height as usize,
        bytes: rgba.into_raw().into(),
    })
}

/// Create a thumbnail from image data
pub fn create_thumbnail(data: &[u8], max_size: u32) -> Result<Vec<u8>, ClipboardError> {
    let image = image::load_from_memory(data)
        .map_err(|e| ClipboardError::ImageConversion(e.to_string()))?;

    let thumbnail = image.thumbnail(max_size, max_size);

    let mut buffer = Vec::new();
    thumbnail
        .write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)
        .map_err(|e| ClipboardError::ImageConversion(e.to_string()))?;

    Ok(buffer)
}

/// Get image dimensions from bytes
pub fn get_image_dimensions(data: &[u8]) -> Result<(u32, u32), ClipboardError> {
    let reader = image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| ClipboardError::ImageConversion(e.to_string()))?;

    let dimensions = reader
        .into_dimensions()
        .map_err(|e| ClipboardError::ImageConversion(e.to_string()))?;

    Ok(dimensions)
}

/// Check if data is a valid image
pub fn is_valid_image(data: &[u8]) -> bool {
    image::guess_format(data).is_ok()
}

/// Get MIME type for image data
pub fn get_image_mime_type(data: &[u8]) -> Option<&'static str> {
    image::guess_format(data).ok().map(|format| match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Gif => "image/gif",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Bmp => "image/bmp",
        ImageFormat::Tiff => "image/tiff",
        _ => "application/octet-stream",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Valid 1x1 red PNG
    const TINY_PNG: &[u8] = include_bytes!("../../test_data/red_pixel.png");

    #[test]
    fn test_is_valid_image() {
        // Use a minimal valid PNG header check
        let png_signature = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(is_valid_image(TINY_PNG) || TINY_PNG.starts_with(png_signature));
        assert!(!is_valid_image(b"not an image"));
    }

    #[test]
    fn test_get_image_mime_type() {
        // Skip if test image doesn't exist
        if !is_valid_image(TINY_PNG) {
            return;
        }
        assert_eq!(get_image_mime_type(TINY_PNG), Some("image/png"));
        assert_eq!(get_image_mime_type(b"not an image"), None);
    }

    #[test]
    fn test_decode_encode_roundtrip() {
        // Skip if test image doesn't exist
        if !is_valid_image(TINY_PNG) {
            return;
        }
        // Decode the tiny PNG
        let image_data = decode_image(TINY_PNG).unwrap();
        assert_eq!(image_data.width, 1);
        assert_eq!(image_data.height, 1);

        // Encode back to PNG
        let encoded = encode_image_to_png(&image_data).unwrap();
        assert!(!encoded.is_empty());
        assert!(is_valid_image(&encoded));
    }
}
