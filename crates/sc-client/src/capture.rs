//! Captures of the output as PNG files: what a cloud session shows the owner (CLAUDE.md 12: it
//! does not measure frame time, it does render), and what the render tests compare.

use std::io::BufWriter;
use std::path::Path;

/// Write `rgba` (rows from the top, 4 bytes a pixel) as a PNG.
pub fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(BufWriter::new(file), width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(std::io::Error::other)?;
    w.write_image_data(rgba).map_err(std::io::Error::other)
}
