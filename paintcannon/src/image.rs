use base64::{engine::general_purpose::STANDARD, Engine};
use std::fs;
use std::io::Cursor;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DecodedImage {
    pub(crate) width_px: u32,
    pub(crate) height_px: u32,
    pub(crate) rgba: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ImageError {
    ReadFailed,
    DecodeFailed,
    EmptyImage,
    UnsupportedColor,
}

pub(crate) fn load_png_image(source: &str) -> Result<DecodedImage, ImageError> {
    let bytes = if let Some(data) = source.strip_prefix("data:image/png;base64,") {
        STANDARD
            .decode(data)
            .map_err(|_| ImageError::DecodeFailed)?
    } else {
        fs::read(source).map_err(|_| ImageError::ReadFailed)?
    };
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
        rgba: decoded_png_to_rgba(&decoded[..output.buffer_size()], output.color_type)?,
    })
}

fn decoded_png_to_rgba(bytes: &[u8], color_type: png::ColorType) -> Result<Vec<u8>, ImageError> {
    let mut rgba = Vec::new();
    match color_type {
        png::ColorType::Rgb => {
            for pixel in bytes.chunks_exact(3) {
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
        }
        png::ColorType::Rgba => rgba.extend_from_slice(bytes),
        png::ColorType::Grayscale => {
            rgba.reserve(bytes.len() * 4);
            for value in bytes {
                rgba.extend_from_slice(&[*value, *value, *value, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            rgba.reserve(bytes.len() * 2);
            for pixel in bytes.chunks_exact(2) {
                let value = pixel[0];
                rgba.extend_from_slice(&[value, value, value, pixel[1]]);
            }
        }
        png::ColorType::Indexed => return Err(ImageError::UnsupportedColor),
    }
    Ok(rgba)
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
    fn decodes_rgb_png_from_data_url() {
        let bytes = encode_png(png::ColorType::Rgb, 2, 1, &[255, 0, 0, 0, 255, 0]);

        assert_eq!(
            load_png_image(&format!(
                "data:image/png;base64,{}",
                STANDARD.encode(&bytes)
            ))
            .unwrap(),
            DecodedImage {
                width_px: 2,
                height_px: 1,
                rgba: vec![255, 0, 0, 255, 0, 255, 0, 255],
            }
        );
    }

    #[test]
    fn decodes_rgba_png_preserving_alpha() {
        let bytes = encode_png(png::ColorType::Rgba, 2, 1, &[255, 0, 0, 1, 0, 255, 0, 255]);

        assert_eq!(
            decode_png_image(&bytes).unwrap().rgba,
            vec![255, 0, 0, 1, 0, 255, 0, 255]
        );
    }

    #[test]
    fn decodes_grayscale_png_to_rgba() {
        let bytes = encode_png(png::ColorType::Grayscale, 2, 1, &[10, 250]);

        assert_eq!(
            decode_png_image(&bytes).unwrap().rgba,
            vec![10, 10, 10, 255, 250, 250, 250, 255]
        );
    }
}
