//! Startup-only soft glow. Stochastic alpha quantization avoids concentric
//! 8-bit gradient bands without any per-frame filter, allocation, or upload.

pub(super) const SIZE: u32 = 768;

pub(super) fn pixels() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let edge = (-4.5_f32).exp();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = (x as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let gaussian = (((-4.5 * (dx * dx + dy * dy)).exp() - edge) / (1.0 - edge)).max(0.0);
            let alpha = gaussian * 0.08 * 255.0;
            let noise = dither(x, y);
            pixels.extend_from_slice(&[255, 255, 255, (alpha + noise).floor() as u8]);
        }
    }
    pixels
}

fn dither(x: u32, y: u32) -> f32 {
    // Deterministic per-pixel hash, not runtime randomness or a visible Bayer grid.
    let mut hash = (y * SIZE + x).wrapping_add(0x9e37_79b9);
    hash = (hash ^ (hash >> 16)).wrapping_mul(0x7feb_352d);
    hash = (hash ^ (hash >> 15)).wrapping_mul(0x846c_a68b);
    hash ^= hash >> 16;
    (hash >> 8) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glow_is_bounded_transparent_at_edges_and_dithered_between_levels() {
        let data = pixels();
        assert_eq!(data.len(), (SIZE * SIZE * 4) as usize);
        assert_eq!(data, pixels());
        let alpha = |x: u32, y: u32| data[((y * SIZE + x) * 4 + 3) as usize];
        assert!(data.chunks_exact(4).all(|pixel| pixel[3] <= 21));
        for edge in 0..SIZE {
            assert_eq!(alpha(edge, 0), 0);
            assert_eq!(alpha(edge, SIZE - 1), 0);
        }
        assert!(alpha(SIZE / 2, SIZE / 2) >= 19);
        // Along a narrow tangential strip the alpha should use adjacent levels,
        // not one solid annulus; their mean preserves the Gaussian intensity.
        let strip: Vec<_> = (370..398).map(|y| alpha(500, y)).collect();
        assert!(strip.windows(2).any(|pair| pair[0] != pair[1]));
        assert!(strip.iter().max().unwrap() - strip.iter().min().unwrap() <= 1);
    }
}
