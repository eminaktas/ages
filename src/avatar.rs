use std::fmt;
use std::path::{Path, PathBuf};

use i18n_embed_fl::fl;

pub const SIZE: u32 = 256;

#[derive(Debug)]
pub enum AvatarError {
    BadImage(PathBuf),
    Write { path: PathBuf, reason: String },
}

impl fmt::Display for AvatarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let l = &crate::i18n::LOADER;
        let s = match self {
            AvatarError::BadImage(p) => fl!(l, "error-bad-image", path = p.display().to_string()),
            AvatarError::Write { path, reason } => fl!(
                l,
                "error-store-write",
                path = path.display().to_string(),
                reason = reason.as_str()
            ),
        };
        f.write_str(&s)
    }
}

impl std::error::Error for AvatarError {}

/// Default mosaic size for `--pixel` without a value.
pub const DEFAULT_PIXEL_GRID: u32 = 32;
/// Palette size for pixel-art avatars.
pub const PIXEL_COLORS: usize = 32;

/// Open `src`, center-crop to a square, resize to `SIZE`×`SIZE`, save as PNG at `dest`.
/// With `pixel = Some(grid)` the avatar is stored as `grid`×`grid` pixel art instead
/// (see the `pixelart` crate); the conversion happens once, here, never at render time.
pub fn import(src: &Path, dest: &Path, pixel: Option<u32>) -> Result<(), AvatarError> {
    let img = image::open(src).map_err(|_| AvatarError::BadImage(src.to_path_buf()))?;
    let resized = match pixel {
        Some(grid) => pixelart::pixelate(
            &img,
            pixelart::Options::default()
                .grid(grid)
                .colors(PIXEL_COLORS)
                .output_size(SIZE),
        ),
        None => {
            let (w, h) = (img.width(), img.height());
            let side = w.min(h);
            let cropped = img.crop_imm((w - side) / 2, (h - side) / 2, side, side);
            cropped.resize_exact(SIZE, SIZE, image::imageops::FilterType::Lanczos3)
        }
    };
    let write_err = |e: String| AvatarError::Write {
        path: dest.to_path_buf(),
        reason: e,
    };
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| write_err(e.to_string()))?;
    }
    resized
        .to_rgba8()
        .save_with_format(dest, image::ImageFormat::Png)
        .map_err(|e| write_err(e.to_string()))
}

/// Cheap check that `path` is a readable file in a format `image` recognizes.
pub fn probe(path: &Path) -> Result<(), AvatarError> {
    let bad = || AvatarError::BadImage(path.to_path_buf());
    let reader = image::ImageReader::open(path).map_err(|_| bad())?;
    let reader = reader.with_guessed_format().map_err(|_| bad())?;
    if reader.format().is_none() {
        return Err(bad());
    }
    Ok(())
}

pub fn load(path: &Path) -> Option<image::DynamicImage> {
    image::open(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn import_non_square_jpeg() {
        let d = tempdir().unwrap();
        let src = d.path().join("in.jpg");
        image::RgbImage::from_fn(300, 100, |x, _| image::Rgb([x as u8, 0, 0]))
            .save(&src)
            .unwrap();
        let dest = d.path().join("avatars/anne.png");
        import(&src, &dest, None).unwrap();
        let out = image::open(&dest).unwrap();
        assert_eq!((out.width(), out.height()), (256, 256));
        assert!(load(&dest).is_some());
    }

    #[test]
    fn import_text_file_fails() {
        let d = tempdir().unwrap();
        let src = d.path().join("x.txt");
        std::fs::write(&src, "hi").unwrap();
        let dest = d.path().join("a.png");
        assert!(matches!(
            import(&src, &dest, None),
            Err(AvatarError::BadImage(_))
        ));
        assert!(!dest.exists());
    }

    #[test]
    fn probe_accepts_image_rejects_text() {
        let d = tempdir().unwrap();
        let png = d.path().join("a.png");
        image::RgbImage::new(4, 4).save(&png).unwrap();
        assert!(probe(&png).is_ok());
        let txt = d.path().join("a.txt");
        std::fs::write(&txt, "hello").unwrap();
        assert!(matches!(probe(&txt), Err(AvatarError::BadImage(_))));
        assert!(probe(Path::new("/nope.png")).is_err());
    }

    #[test]
    fn import_pixel_stores_blocky_low_color_png() {
        let d = tempdir().unwrap();
        let src = d.path().join("in.png");
        image::RgbImage::from_fn(200, 200, |x, y| image::Rgb([x as u8, y as u8, 90]))
            .save(&src)
            .unwrap();
        let dest = d.path().join("a.png");
        import(&src, &dest, Some(16)).unwrap();
        let out = image::open(&dest).unwrap().to_rgba8();
        assert_eq!((out.width(), out.height()), (SIZE, SIZE));
        let colors: std::collections::HashSet<[u8; 4]> = out.pixels().map(|p| p.0).collect();
        assert!(colors.len() <= PIXEL_COLORS, "{}", colors.len());
        let cell = SIZE / 16;
        for x in 0..cell {
            assert_eq!(out.get_pixel(x, 0), out.get_pixel(0, 0));
        }
    }

    #[test]
    fn import_missing_fails() {
        let d = tempdir().unwrap();
        assert!(import(Path::new("/nope/none.png"), &d.path().join("x.png"), None).is_err());
    }
}
