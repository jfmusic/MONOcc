//! the base screen-image codec: a trimmed 4-bit palette format.
//!
//! a `.raw` screen image is a 16-color palette (48 bytes, each color stored
//! blue-green-red) followed by a 128x128 body of 4-bit palette indices packed
//! two per byte, with the body stored in reverse byte order. encoding from a
//! png quantizes to the 16 most frequent colors.
//!
//! the quantizer reproduces a quirk of the original: pygame constructed its
//! comparison colors from a packed integer interpreted as rgba, which shifted
//! every channel and dropped blue out of the distance metric. nearest-color is
//! therefore measured on the red and green channels only. this is preserved so
//! encoded bytes match the committed reference assets exactly.

use image::GenericImageView;

pub const IMAGE_DIM: u32 = 128;
const PALETTE_COLORS: usize = 16;
const PALETTE_BYTES: usize = PALETTE_COLORS * 3;
const BODY_BYTES: usize = (IMAGE_DIM as usize / 2) * IMAGE_DIM as usize;
pub const RAW_BYTES: usize = PALETTE_BYTES + BODY_BYTES;

/// a decoded screen image: its 16-entry rgb palette and its palette-index body.
pub struct RawImage {
    pub palette: Vec<(u8, u8, u8)>,
    pub pixels: Vec<u8>,
}

/// decode a `.raw` screen image into its palette and palette-index pixels.
///
/// palette bytes are stored blue-green-red; the returned tuples are rgb. body
/// bytes are read high-nibble then low-nibble, matching the original reader.
pub fn decode(data: &[u8]) -> RawImage {
    let mut palette = Vec::with_capacity(PALETTE_COLORS);
    for i in 0..PALETTE_COLORS {
        let b = data[i * 3];
        let g = data[i * 3 + 1];
        let r = data[i * 3 + 2];
        palette.push((r, g, b));
    }

    let mut pixels = Vec::with_capacity(BODY_BYTES * 2);
    for &byte in &data[PALETTE_BYTES..PALETTE_BYTES + BODY_BYTES] {
        pixels.push(byte >> 4);
        pixels.push(byte & 0x0f);
    }

    RawImage { palette, pixels }
}

/// pack a color into the surface integer used by the quantizer (`0x00RRGGBB`).
fn pack(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

/// encode png bytes into the trimmed 4bpp `.raw` screen-image format.
pub fn encode_png(png: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    let img = image::load_from_memory(png)?;
    let img = if img.dimensions() == (IMAGE_DIM, IMAGE_DIM) {
        img
    } else {
        img.resize_exact(IMAGE_DIM, IMAGE_DIM, image::imageops::FilterType::Nearest)
    };
    let rgb = img.to_rgb8();

    // scan pixels right-to-left within each top-to-bottom row, packing each
    // color and tallying frequency in first-seen order.
    let mut pixels: Vec<u32> = Vec::with_capacity((IMAGE_DIM * IMAGE_DIM) as usize);
    let mut counts: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    let mut order: Vec<u32> = Vec::new();
    for y in 0..IMAGE_DIM {
        for x in (0..IMAGE_DIM).rev() {
            let p = rgb.get_pixel(x, y);
            let c = pack(p[0], p[1], p[2]);
            pixels.push(c);
            counts.entry(c).and_modify(|n| *n += 1).or_insert_with(|| {
                order.push(c);
                1
            });
        }
    }

    // index 0 is forced to black; the remaining slots take the most frequent
    // colors, ties broken by first-seen order (a stable sort).
    order.sort_by(|a, b| counts[b].cmp(&counts[a]));
    let mut top = vec![0u32];
    top.extend(order);
    top.truncate(PALETTE_COLORS);

    let mut palette = Vec::with_capacity(PALETTE_BYTES);
    let mut palette_map: std::collections::HashMap<u32, u8> = std::collections::HashMap::new();
    for (i, &c) in top.iter().enumerate() {
        palette.push((c & 0xff) as u8);
        palette.push((c >> 8 & 0xff) as u8);
        palette.push((c >> 16 & 0xff) as u8);
        palette_map.insert(c, i as u8);
    }
    palette.resize(PALETTE_BYTES, 0);

    // map every pixel to its nearest palette color over the red/green channels,
    // ties broken by the smaller packed color value.
    let mut body: Vec<u8> = Vec::with_capacity(BODY_BYTES);
    let mut nibble: Option<u8> = None;
    for &c in &pixels {
        let pr = (c >> 16 & 0xff) as i64;
        let pg = (c >> 8 & 0xff) as i64;
        let index = palette_map
            .iter()
            .map(|(&pc, &idx)| {
                let cr = (pc >> 16 & 0xff) as i64;
                let cg = (pc >> 8 & 0xff) as i64;
                let dist = (pr - cr).pow(2) + (pg - cg).pow(2);
                (dist, pc, idx)
            })
            .min_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)))
            .map(|(_, _, idx)| idx)
            .unwrap_or(0);

        match nibble.take() {
            None => nibble = Some(index),
            Some(low) => body.push(index << 4 | low),
        }
    }

    body.reverse();
    let mut data = palette;
    data.extend(body);
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources;

    #[test]
    fn decode_reads_palette_and_body() {
        let raw = resources::resource_bytes("assets/palettegear/23.raw").unwrap();
        let image = decode(&raw);
        assert_eq!(PALETTE_COLORS, image.palette.len());
        assert_eq!((IMAGE_DIM * IMAGE_DIM) as usize, image.pixels.len());
        // slot 23 is the black base screen: every palette entry is black.
        assert!(image.palette.iter().all(|&c| c == (0, 0, 0)));
        assert!(image.pixels.iter().all(|&p| p < PALETTE_COLORS as u8));
    }

    #[test]
    fn encode_png_matches_reference_raw() {
        let png = resources::resource_bytes("assets/palettegear/23.png").unwrap();
        let raw = resources::resource_bytes("assets/palettegear/23.raw").unwrap();
        let encoded = encode_png(&png).unwrap();
        assert_eq!(RAW_BYTES, encoded.len());
        assert_eq!(raw.as_ref(), encoded.as_slice());
    }
}
