use crate::backend::DesktopBackend;
use crate::protocol::Region;

pub async fn pick_color(
    backend: &dyn DesktopBackend,
    x: u32,
    y: u32,
    path: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let (source_path, sample_x, sample_y) = if let Some(path) = path {
        (path.to_string(), x, y)
    } else {
        let screenshot = backend
            .screenshot(
                None,
                Some(Region {
                    x,
                    y,
                    width: 1,
                    height: 1,
                }),
                None,
            )
            .await?;
        (screenshot.path, 0, 0)
    };
    let pixel = tokio::task::spawn_blocking({
        let source_path = source_path.clone();
        move || sample_pixel(&source_path, sample_x, sample_y)
    })
    .await??;

    Ok(serde_json::json!({
        "x": x,
        "y": y,
        "source_path": source_path,
        "red": pixel[0],
        "green": pixel[1],
        "blue": pixel[2],
        "alpha": pixel[3],
        "hex": rgba_to_hex(pixel)
    }))
}

fn sample_pixel(path: &str, x: u32, y: u32) -> anyhow::Result<[u8; 4]> {
    let image = image::open(path)?.to_rgba8();
    if x >= image.width() || y >= image.height() {
        anyhow::bail!(
            "sample coordinate {},{} outside image bounds {}x{}",
            x,
            y,
            image.width(),
            image.height()
        );
    }
    Ok(image.get_pixel(x, y).0)
}

fn rgba_to_hex(pixel: [u8; 4]) -> String {
    if pixel[3] == 255 {
        format!("#{:02x}{:02x}{:02x}", pixel[0], pixel[1], pixel[2])
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            pixel[0], pixel[1], pixel[2], pixel[3]
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats_rgb_and_rgba_hex() {
        assert_eq!(super::rgba_to_hex([255, 128, 0, 255]), "#ff8000");
        assert_eq!(super::rgba_to_hex([255, 128, 0, 127]), "#ff80007f");
    }
}
