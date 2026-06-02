use crate::protocol::Region;
use anyhow::Context;

#[derive(Clone, Copy)]
pub(crate) struct CropRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl CropRect {
    pub fn from_region(region: &Region) -> Self {
        Self {
            x: region.x as i32,
            y: region.y as i32,
            width: region.width,
            height: region.height,
        }
    }

    pub fn to_grim_geometry(self) -> String {
        format!("{}x{}+{}+{}", self.width, self.height, self.x, self.y)
    }
}

pub(crate) async fn get_png_dimensions(path: &str) -> anyhow::Result<(u32, u32)> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut header = [0u8; 24];
    file.read_exact(&mut header).await?;
    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    Ok((width, height))
}

pub(crate) fn crop_png(path: &str, rect: CropRect) -> anyhow::Result<()> {
    let image = image::open(path)
        .with_context(|| format!("opening screenshot for crop: {}", path))?
        .to_rgba8();
    let (image_width, image_height) = image.dimensions();

    if rect.x < 0
        || rect.y < 0
        || rect.width == 0
        || rect.height == 0
        || rect.x as u64 + rect.width as u64 > image_width as u64
        || rect.y as u64 + rect.height as u64 > image_height as u64
    {
        anyhow::bail!(
            "requested screenshot crop {}x{}+{}+{} is outside captured image {}x{}",
            rect.width,
            rect.height,
            rect.x,
            rect.y,
            image_width,
            image_height
        );
    }

    let cropped = image::imageops::crop_imm(
        &image,
        rect.x as u32,
        rect.y as u32,
        rect.width,
        rect.height,
    )
    .to_image();
    cropped
        .save(path)
        .with_context(|| format!("saving cropped screenshot: {}", path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CropRect, crop_png, get_png_dimensions};
    use image::{ImageBuffer, Rgba};

    fn temp_png_path(name: &str) -> String {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("/tmp/deskbrid_{}_{}_{}.png", name, std::process::id(), ts)
    }

    #[test]
    fn crop_png_rewrites_file_to_requested_region() {
        let path = temp_png_path("crop");
        let image = ImageBuffer::from_fn(4, 3, |x, y| Rgba([x as u8, y as u8, 0, 255]));
        image.save(&path).unwrap();

        crop_png(
            &path,
            CropRect {
                x: 1,
                y: 1,
                width: 2,
                height: 1,
            },
        )
        .unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        assert_eq!(rt.block_on(get_png_dimensions(&path)).unwrap(), (2, 1));
        let cropped = image::open(&path).unwrap().to_rgba8();
        assert_eq!(cropped.get_pixel(0, 0).0, [1, 1, 0, 255]);
        assert_eq!(cropped.get_pixel(1, 0).0, [2, 1, 0, 255]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn crop_png_rejects_out_of_bounds_region() {
        let path = temp_png_path("crop_oob");
        let image = ImageBuffer::from_pixel(2, 2, Rgba([0u8, 0, 0, 255]));
        image.save(&path).unwrap();

        let err = crop_png(
            &path,
            CropRect {
                x: 1,
                y: 1,
                width: 2,
                height: 1,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("outside captured image"));

        let _ = std::fs::remove_file(path);
    }
}
