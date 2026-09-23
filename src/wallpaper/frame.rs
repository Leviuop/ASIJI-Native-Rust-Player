use crate::render::Mode;
use anyhow::Result;

pub fn bmp(rgb: &[u8], dimensions: (usize, usize), mode: Mode, color: bool) -> Result<Vec<u8>> {
    let (cols, rows) = dimensions;
    anyhow::ensure!(
        (1..=1000).contains(&cols) && (1..=1000).contains(&rows),
        "Неверный размер кадра обоев"
    );
    let blocks = mode == Mode::Blocks;
    let source_height = rows * if blocks { 2 } else { 1 };
    anyhow::ensure!(
        rgb.len() == cols * source_height * 3,
        "Неверная длина кадра обоев"
    );
    let (width, height) = if blocks {
        (cols, rows * 2)
    } else {
        (cols * 5, rows * 10)
    };
    let stride = (width * 3).next_multiple_of(4);
    let mut out = vec![0; 54 + stride * height];
    let length = out.len() as u32;
    out[..2].copy_from_slice(b"BM");
    out[2..6].copy_from_slice(&length.to_le_bytes());
    out[10..14].copy_from_slice(&54_u32.to_le_bytes());
    out[14..18].copy_from_slice(&40_u32.to_le_bytes());
    out[18..22].copy_from_slice(&(width as i32).to_le_bytes());
    out[22..26].copy_from_slice(&(-(height as i32)).to_le_bytes());
    out[26..28].copy_from_slice(&1_u16.to_le_bytes());
    out[28..30].copy_from_slice(&24_u16.to_le_bytes());
    // Original 5×7 masks for " .:-=+*#%@", placed in 5×10 terminal cells.
    // No external font file or font rasterizer is bundled.
    const GLYPHS: [[u8; 7]; 10] = [
        [0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 4, 4],
        [0, 4, 4, 0, 4, 4, 0],
        [0, 0, 0, 31, 0, 0, 0],
        [0, 0, 31, 0, 31, 0, 0],
        [4, 4, 4, 31, 4, 4, 4],
        [4, 21, 14, 31, 14, 21, 4],
        [10, 31, 10, 10, 31, 10, 10],
        [25, 26, 4, 4, 4, 11, 19],
        [14, 17, 23, 21, 23, 16, 15],
    ];
    for (i, p) in rgb.chunks_exact(3).enumerate() {
        let gray = ((77 * p[0] as u32 + 150 * p[1] as u32 + 29 * p[2] as u32 + 128) >> 8) as u8;
        let pixel = if color { [p[2], p[1], p[0]] } else { [gray; 3] };
        let (x, y) = (i % cols, i / cols);
        if blocks {
            let offset = 54 + y * stride + x * 3;
            out[offset..offset + 3].copy_from_slice(&pixel);
        } else {
            let glyph = &GLYPHS[((gray as f32 / 255.0).powf(0.85) * 9.0) as usize];
            for (gy, mask) in glyph.iter().enumerate() {
                for gx in 0..5 {
                    if mask & (1 << (4 - gx)) != 0 {
                        let offset = 54 + (y * 10 + gy + 1) * stride + (x * 5 + gx) * 3;
                        out[offset..offset + 3].copy_from_slice(&pixel);
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bmp_keeps_color_orientation_padding_and_ascii_cells() {
        let pixels = [255, 0, 0, 0, 0, 255];
        let image = bmp(&pixels, (1, 1), Mode::Blocks, true).unwrap();
        assert_eq!(&image[..2], b"BM");
        assert_eq!(i32::from_le_bytes(image[22..26].try_into().unwrap()), -2);
        assert_eq!(&image[54..], &[0, 0, 255, 0, 255, 0, 0, 0]);
        let mono = bmp(&pixels, (1, 1), Mode::Blocks, false).unwrap();
        assert_eq!(mono[54], mono[55]);
        assert_eq!(mono[55], mono[56]);
        let ascii = bmp(&[255; 3], (1, 1), Mode::Ascii, true).unwrap();
        assert_eq!(i32::from_le_bytes(ascii[18..22].try_into().unwrap()), 5);
        assert_eq!(i32::from_le_bytes(ascii[22..26].try_into().unwrap()), -10);
        assert!(ascii[54..].contains(&255) && ascii[54..].contains(&0));
        assert!(bmp(&[1, 2], (1, 1), Mode::Ascii, true).is_err());
        assert!(bmp(&[], (0, 0), Mode::Blocks, true).is_err());
    }
}
