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

/// Open `src`, center-crop to a square, resize to `SIZE`×`SIZE`, save as PNG at `dest`.
pub fn import(src: &Path, dest: &Path) -> Result<(), AvatarError> {
    let img = image::open(src).map_err(|_| AvatarError::BadImage(src.to_path_buf()))?;
    let (w, h) = (img.width(), img.height());
    let side = w.min(h);
    let cropped = img.crop_imm((w - side) / 2, (h - side) / 2, side, side);
    let resized = cropped.resize_exact(SIZE, SIZE, image::imageops::FilterType::Lanczos3);
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
        import(&src, &dest).unwrap();
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
        assert!(matches!(import(&src, &dest), Err(AvatarError::BadImage(_))));
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
    fn import_missing_fails() {
        let d = tempdir().unwrap();
        assert!(import(Path::new("/nope/none.png"), &d.path().join("x.png")).is_err());
    }
}
