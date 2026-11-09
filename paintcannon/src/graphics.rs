use std::io::{self, Write};
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{imageops, ImageBuffer, Rgba};

use crate::frame::ClipRect;
use crate::style::Background;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphicsProtocol {
    Iterm2,
    Kitty,
}

#[derive(Clone, Debug)]
pub(crate) struct GraphicsImage {
    pub(crate) rgba: Arc<[u8]>,
    pub(crate) width_px: u32,
    pub(crate) height_px: u32,
    pub(crate) cell_width_px: u32,
    pub(crate) cell_height_px: u32,
    pub(crate) bounds: ClipRect,
    pub(crate) background: Background,
}

impl PartialEq for GraphicsImage {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.rgba, &other.rgba)
            && self.width_px == other.width_px
            && self.height_px == other.height_px
            && self.cell_width_px == other.cell_width_px
            && self.cell_height_px == other.cell_height_px
            && self.bounds == other.bounds
            && self.background == other.background
    }
}

impl Eq for GraphicsImage {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ImageFragment {
    pub(crate) image: GraphicsImage,
    pub(crate) rect: ClipRect,
}

impl ImageFragment {
    fn encode_png(&self) -> io::Result<Vec<u8>> {
        let image = &self.image;
        let x = (self.rect.left - image.bounds.left) as u64;
        let y = (self.rect.top - image.bounds.top) as u64;
        let width = image.bounds.width() as u64;
        let height = image.bounds.height() as u64;
        let left = (x * u64::from(image.width_px) / width) as u32;
        let top = (y * u64::from(image.height_px) / height) as u32;
        let right = ((x + self.rect.width() as u64) * u64::from(image.width_px))
            .div_ceil(width)
            .min(u64::from(image.width_px)) as u32;
        let bottom = ((y + self.rect.height() as u64) * u64::from(image.height_px))
            .div_ceil(height)
            .min(u64::from(image.height_px)) as u32;
        let source = ImageBuffer::<Rgba<u8>, _>::from_raw(
            image.width_px,
            image.height_px,
            image.rgba.as_ref(),
        )
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid image dimensions"))?;
        let cropped = imageops::crop_imm(&source, left, top, right - left, bottom - top);
        let scale = (self.rect.width() as f64 * image.cell_width_px as f64 / (right - left) as f64)
            .min(self.rect.height() as f64 * image.cell_height_px as f64 / (bottom - top) as f64)
            .min(1.0);
        let width = ((right - left) as f64 * scale).round().max(1.0) as u32;
        let height = ((bottom - top) as f64 * scale).round().max(1.0) as u32;
        let pixels = imageops::resize(&*cropped, width, height, imageops::FilterType::Lanczos3);
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(pixels.as_raw())?;
        }
        Ok(png)
    }

    pub(crate) fn write_to(
        &self,
        out: &mut impl Write,
        protocol: GraphicsProtocol,
        index: usize,
    ) -> io::Result<()> {
        let data = STANDARD.encode(self.encode_png()?);
        write!(out, "\x1b[{};{}H", self.rect.top + 1, self.rect.left + 1)?;
        match protocol {
            GraphicsProtocol::Iterm2 => {
                write!(
                    out,
                    "\x1b]1337;File=inline=1;width={};height={};preserveAspectRatio=1:{}\x07",
                    self.rect.width(),
                    self.rect.height(),
                    data
                )
            }
            // https://sw.kovidgoyal.net/kitty/graphics-protocol/#display-images-on-screen:
            // Reusing both image and placement IDs replaces the placement.
            GraphicsProtocol::Kitty => {
                let mut chunks = data.as_bytes().chunks(4096).peekable();
                let mut first = true;
                while let Some(chunk) = chunks.next() {
                    let more = u8::from(chunks.peek().is_some());
                    if first {
                        write!(
                            out,
                            "\x1b_Ga=T,f=100,t=d,i={},p=1,c={},r={},C=1,q=2,m={more};",
                            0x50430000 + index,
                            self.rect.width(),
                            self.rect.height()
                        )?;
                        first = false;
                    } else {
                        write!(out, "\x1b_Gm={more},q=2;")?;
                    }
                    out.write_all(chunk)?;
                    out.write_all(b"\x1b\\")?;
                }
                Ok(())
            }
        }
    }
}

pub(crate) fn delete_images(
    out: &mut impl Write,
    protocol: GraphicsProtocol,
    count: usize,
) -> io::Result<()> {
    if protocol == GraphicsProtocol::Kitty {
        for index in 0..count {
            write!(out, "\x1b_Ga=d,d=I,i={},q=2\x1b\\", 0x50430000 + index)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_thumbnails_filter_detail_before_transmission() {
        let rgba: Vec<u8> = (0..64)
            .flat_map(|index| {
                let value = if (index / 8 + index % 8) % 2 == 0 {
                    0
                } else {
                    255
                };
                [value, value, value, 255]
            })
            .collect();
        let fragment = ImageFragment {
            image: GraphicsImage {
                rgba: rgba.into(),
                width_px: 8,
                height_px: 8,
                cell_width_px: 1,
                cell_height_px: 1,
                bounds: ClipRect::new(0, 0, 2, 2),
                background: Background::Black,
            },
            rect: ClipRect::new(0, 0, 2, 2),
        };
        let decoded = crate::image::decode_png_image(&fragment.encode_png().unwrap()).unwrap();
        assert_eq!((decoded.width_px, decoded.height_px), (2, 2));
        for pixel in decoded.rgba.chunks_exact(4) {
            assert!((120..=135).contains(&pixel[0]));
            assert_eq!(pixel[0], pixel[1]);
            assert_eq!(pixel[1], pixel[2]);
            assert_eq!(pixel[3], 255);
        }
    }
}
