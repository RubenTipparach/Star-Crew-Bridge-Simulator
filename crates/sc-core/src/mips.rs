//! Texture layers made smaller: the box filter and the mip chain every texture array is uploaded with
//! (openspec/changes/surface-materials). It lives in the core because `deckc` packs the rooms' and the hull's arrays with
//! it and the client builds the stations' arrays with it (patrol-mission design 12b): one filter, so a layer looks the
//! same however far off it is in either (CLAUDE.md 6.1). A smaller layer joins a bigger array by `grow`.

/// Box-filter `src` (square RGBA8 layers of `from` px) to `to` px, `to` dividing `from`.
pub fn shrink(src: &[u8], from: u32, to: u32) -> Vec<u8> {
    let k = (from / to) as usize;
    let (f, t) = (from as usize, to as usize);
    let mut out = vec![0u8; t * t * 4];
    for y in 0..t {
        for x in 0..t {
            let mut acc = [0u32; 4];
            for dy in 0..k {
                for dx in 0..k {
                    let i = ((y * k + dy) * f + x * k + dx) * 4;
                    for (c, a) in acc.iter_mut().enumerate() {
                        *a += u32::from(src[i + c]);
                    }
                }
            }
            for c in 0..4 {
                out[(y * t + x) * 4 + c] = (acc[c] / (k * k) as u32) as u8;
            }
        }
    }
    out
}

/// A texture array's mip chain: level 0 is `level0` (`layers` square RGBA8 layers of `size` px), each next level every
/// layer box-filtered to half the size, down to 1 px.
pub fn mip_chain(level0: Vec<u8>, size: u32, layers: u32) -> Vec<Vec<u8>> {
    let mut levels = vec![level0];
    let mut px = size;
    while px > 1 {
        let lb = (px * px * 4) as usize;
        let last = levels.last().unwrap();
        let next = (0..layers as usize).flat_map(|i| shrink(&last[i * lb..(i + 1) * lb], px, px / 2)).collect();
        levels.push(next);
        px /= 2;
    }
    levels
}

/// Repeat each texel of `src` (one square RGBA8 layer of `from` px) into a square block, to `to` px, `from` dividing
/// `to`: a small layer joining an array of bigger ones, its texels kept as they are.
pub fn grow(src: &[u8], from: u32, to: u32) -> Vec<u8> {
    let k = (to / from) as usize;
    let (f, t) = (from as usize, to as usize);
    let mut out = vec![0u8; t * t * 4];
    for y in 0..t {
        for x in 0..t {
            let s = ((y / k) * f + x / k) * 4;
            out[(y * t + x) * 4..(y * t + x) * 4 + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layer_halves_by_averaging_each_two_by_two_block() {
        let src: Vec<u8> = [[0u8, 0, 0, 255], [255, 255, 255, 255]].iter().cycle().take(4).flatten().copied().collect();
        assert_eq!(shrink(&src, 2, 1), vec![127, 127, 127, 255]);
    }

    #[test]
    fn a_layer_grows_by_repeating_each_texel_and_shrinks_back_to_itself() {
        let src: Vec<u8> = (0..16u8).collect();
        let big = grow(&src, 2, 4);
        assert_eq!(&big[0..8], &[0, 1, 2, 3, 0, 1, 2, 3], "the first texel twice across");
        assert_eq!(shrink(&big, 4, 2), src);
    }

    #[test]
    fn a_chain_goes_down_to_one_pixel_for_every_layer() {
        let levels = mip_chain(vec![200u8; 8 * 8 * 4 * 2], 8, 2);
        assert_eq!(levels.iter().map(Vec::len).collect::<Vec<_>>(), [512, 128, 32, 8]);
    }
}
