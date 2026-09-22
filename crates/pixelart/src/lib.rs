//! Pixel art from photos.
//!
//! The pipeline is deliberately simple and fast:
//!
//! 1. **Downsample** the source to a `grid × grid` mosaic with an area-averaging
//!    (triangle) filter, so each cell is the mean of the pixels it covers instead of a
//!    single noisy sample.
//! 2. **Quantize** the mosaic to an adaptive palette of at most `colors` entries using
//!    NeuQuant ([`color_quant`]), which keeps skin tones and dominant hues intact.
//! 3. **Upscale** with nearest-neighbour to the requested output size, giving crisp,
//!    hard-edged blocks.
//!
//! On a 256×256 input the whole pipeline takes a few milliseconds.

use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};

/// Parameters for [`pixelate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Number of cells along each axis of the mosaic. Clamped to `1..=1024`.
    pub grid: u32,
    /// Maximum palette size. Clamped to `2..=256`.
    pub colors: usize,
    /// Side length of the square output image. `0` keeps the `grid` size (no upscale).
    pub output_size: u32,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            grid: 32,
            colors: 32,
            output_size: 256,
        }
    }
}

impl Options {
    pub fn grid(mut self, grid: u32) -> Self {
        self.grid = grid;
        self
    }

    pub fn colors(mut self, colors: usize) -> Self {
        self.colors = colors;
        self
    }

    pub fn output_size(mut self, size: u32) -> Self {
        self.output_size = size;
        self
    }
}

/// Convert `img` into pixel art. Non-square input is center-cropped to a square first.
pub fn pixelate(img: &DynamicImage, opts: Options) -> DynamicImage {
    let grid = opts.grid.clamp(1, 1024);
    let colors = opts.colors.clamp(2, 256);
    let mosaic = quantize(&downsample(img, grid), colors);
    let out = if opts.output_size == 0 {
        mosaic
    } else {
        image::imageops::resize(
            &mosaic,
            opts.output_size,
            opts.output_size,
            FilterType::Nearest,
        )
    };
    DynamicImage::ImageRgba8(out)
}

/// Center-crop to a square and shrink to `grid × grid` with area averaging.
pub fn downsample(img: &DynamicImage, grid: u32) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let side = w.min(h).max(1);
    let square = img.crop_imm((w - side) / 2, (h - side) / 2, side, side);
    // Triangle averages the covered source pixels; Nearest would alias, Lanczos would ring.
    square
        .resize_exact(grid, grid, FilterType::Triangle)
        .to_rgba8()
}

/// Snap every pixel of `img` to an adaptive palette of at most `colors` entries.
/// Fully transparent pixels stay transparent.
pub fn quantize(img: &RgbaImage, colors: usize) -> RgbaImage {
    let colors = colors.clamp(2, 256);
    let mut out = img.clone();
    // Sample factor 1 = look at every pixel; the input is tiny so this is still instant.
    let nq = color_quant::NeuQuant::new(1, colors, img.as_raw());
    for px in out.pixels_mut() {
        if px.0[3] == 0 {
            continue;
        }
        nq.map_pixel(&mut px.0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn gradient(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([x as u8, y as u8, 128])
        }))
    }

    fn distinct(img: &RgbaImage) -> usize {
        img.pixels().map(|p| p.0).collect::<HashSet<_>>().len()
    }

    #[test]
    fn output_size_and_palette_bound() {
        let out = pixelate(&gradient(256, 256), Options::default()).to_rgba8();
        assert_eq!((out.width(), out.height()), (256, 256));
        assert!(distinct(&out) <= 32, "{}", distinct(&out));
        assert!(distinct(&out) > 4, "palette collapsed: {}", distinct(&out));
    }

    #[test]
    fn cells_are_uniform_blocks() {
        let opts = Options::default().grid(16).output_size(64);
        let out = pixelate(&gradient(256, 256), opts).to_rgba8();
        let cell = 64 / 16;
        for cy in 0..16 {
            for cx in 0..16 {
                let first = out.get_pixel(cx * cell, cy * cell);
                for dy in 0..cell {
                    for dx in 0..cell {
                        assert_eq!(out.get_pixel(cx * cell + dx, cy * cell + dy), first);
                    }
                }
            }
        }
        // Not a flat image.
        assert_ne!(out.get_pixel(0, 0), out.get_pixel(63, 63));
    }

    #[test]
    fn non_square_is_center_cropped() {
        let out = pixelate(&gradient(300, 100), Options::default().grid(10)).to_rgba8();
        assert_eq!((out.width(), out.height()), (256, 256));
    }

    #[test]
    fn zero_output_size_keeps_grid() {
        let out = pixelate(&gradient(64, 64), Options::default().grid(8).output_size(0));
        assert_eq!((out.width(), out.height()), (8, 8));
    }

    #[test]
    fn transparent_pixels_survive() {
        let mut img = RgbaImage::from_pixel(8, 8, image::Rgba([200, 10, 10, 255]));
        img.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
        let out = quantize(&img, 4);
        assert_eq!(out.get_pixel(0, 0).0[3], 0);
        assert_eq!(out.get_pixel(7, 7).0[3], 255);
    }

    #[test]
    fn options_are_clamped() {
        let out = pixelate(
            &gradient(32, 32),
            Options {
                grid: 0,
                colors: 0,
                output_size: 0,
            },
        );
        assert_eq!((out.width(), out.height()), (1, 1));
    }
}
