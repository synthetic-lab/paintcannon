#![allow(dead_code)]

use std::fs;
use std::io::Cursor;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DecodedImage {
    pub(crate) width_px: u32,
    pub(crate) height_px: u32,
    pub(crate) rgb: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ImageError {
    ReadFailed,
    DecodeFailed,
    EmptyImage,
    UnsupportedColor,
}

pub(crate) fn load_png_image(path: impl AsRef<Path>) -> Result<DecodedImage, ImageError> {
    let bytes = fs::read(path).map_err(|_| ImageError::ReadFailed)?;
    decode_png_image(&bytes)
}

pub(crate) fn decode_png_image(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| ImageError::DecodeFailed)?;
    let mut decoded = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or(ImageError::DecodeFailed)?
    ];
    let output = reader
        .next_frame(&mut decoded)
        .map_err(|_| ImageError::DecodeFailed)?;
    if output.width == 0 || output.height == 0 {
        return Err(ImageError::EmptyImage);
    }

    Ok(DecodedImage {
        width_px: output.width,
        height_px: output.height,
        rgb: decoded_png_to_rgb(&decoded[..output.buffer_size()], output.color_type)?,
    })
}

fn decoded_png_to_rgb(bytes: &[u8], color_type: png::ColorType) -> Result<Vec<u8>, ImageError> {
    let mut rgb = Vec::new();
    match color_type {
        png::ColorType::Rgb => {
            rgb.extend_from_slice(bytes);
        }
        png::ColorType::Rgba => {
            rgb.reserve(bytes.len() / 4 * 3);
            for pixel in bytes.chunks_exact(4) {
                rgb.extend_from_slice(&pixel[..3]);
            }
        }
        png::ColorType::Grayscale => {
            rgb.reserve(bytes.len() * 3);
            for value in bytes {
                rgb.extend_from_slice(&[*value, *value, *value]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            rgb.reserve(bytes.len() / 2 * 3);
            for pixel in bytes.chunks_exact(2) {
                let value = pixel[0];
                rgb.extend_from_slice(&[value, value, value]);
            }
        }
        png::ColorType::Indexed => return Err(ImageError::UnsupportedColor),
    }
    Ok(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_png(color_type: png::ColorType, width: u32, height: u32, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(color_type);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(data).unwrap();
        }
        bytes
    }

    #[test]
    fn decodes_rgb_png() {
        let bytes = encode_png(png::ColorType::Rgb, 2, 1, &[255, 0, 0, 0, 255, 0]);

        assert_eq!(
            decode_png_image(&bytes).unwrap(),
            DecodedImage {
                width_px: 2,
                height_px: 1,
                rgb: vec![255, 0, 0, 0, 255, 0],
            }
        );
    }

    #[test]
    fn decodes_rgba_png_by_dropping_alpha() {
        let bytes = encode_png(png::ColorType::Rgba, 2, 1, &[255, 0, 0, 1, 0, 255, 0, 255]);

        assert_eq!(
            decode_png_image(&bytes).unwrap().rgb,
            vec![255, 0, 0, 0, 255, 0]
        );
    }

    #[test]
    fn decodes_grayscale_png_to_rgb() {
        let bytes = encode_png(png::ColorType::Grayscale, 2, 1, &[10, 250]);

        assert_eq!(
            decode_png_image(&bytes).unwrap().rgb,
            vec![10, 10, 10, 250, 250, 250]
        );
    }
}
