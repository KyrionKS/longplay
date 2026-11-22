//! Cover based palette extraction.
//!
//! Dominant hue of the artwork drives the vinyl tint, the label stays warm
//! cream, the accent is used for small highlights.

use image::imageops::FilterType;

use super::model::Palette;

const HUE_BUCKETS: usize = 12;

pub fn from_bytes(bytes: &[u8]) -> Option<Palette> {
    let image = image::load_from_memory(bytes).ok()?;
    let small = image.resize_exact(48, 48, FilterType::Triangle).to_rgb8();

    let mut weight = [0.0_f64; HUE_BUCKETS];
    let mut sums = [[0.0_f64; 3]; HUE_BUCKETS];

    for pixel in small.pixels() {
        let [r, g, b] = pixel.0;
        let (h, s, l) = rgb_to_hsl(r, g, b);
        if l < 0.06 || l > 0.96 || s < 0.04 {
            continue;
        }
        let bucket = ((h / 30.0).floor() as usize).min(HUE_BUCKETS - 1);
        let w = f64::from(s).powf(1.4) * (1.0 - (f64::from(l) - 0.5).abs() * 1.4).max(0.05);
        weight[bucket] += w;
        sums[bucket][0] += f64::from(r) * w;
        sums[bucket][1] += f64::from(g) * w;
        sums[bucket][2] += f64::from(b) * w;
    }

    let best = weight
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .filter(|(_, w)| **w > 0.5)?;

    let (bucket, total) = best;
    let rgb = [
        (sums[bucket][0] / total).round() as u8,
        (sums[bucket][1] / total).round() as u8,
        (sums[bucket][2] / total).round() as u8,
    ];

    Some(from_rgb(rgb))
}

pub fn from_rgb(rgb: [u8; 3]) -> Palette {
    let (h, s, l) = rgb_to_hsl(rgb[0], rgb[1], rgb[2]);
    let sat = (s * 1.15).clamp(0.18, 0.80);

    Palette {
        vinyl: hsl_hex(h, sat, l.clamp(0.30, 0.46)),
        vinyl_light: hsl_hex(h, sat * 0.9, (l + 0.22).clamp(0.50, 0.66)),
        label: rgb_hex(mix(rgb, [244, 238, 218], 0.84)),
        accent: hsl_hex(h, (s * 1.2).clamp(0.30, 0.90), l.clamp(0.45, 0.62)),
        dark: hsl_hex(h, sat * 0.7, 0.12),
    }
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let f = |x: u8, y: u8| (f32::from(x) * (1.0 - t) + f32::from(y) * t).round() as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])]
}

fn rgb_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn hsl_hex(h: f32, s: f32, l: f32) -> String {
    rgb_hex(hsl_to_rgb(h, s, l))
}

/// Returns hue in degrees, saturation and lightness in `0..=1`.
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = f32::from(r) / 255.0;
    let gf = f32::from(g) / 255.0;
    let bf = f32::from(b) / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if max == rf {
        ((gf - bf) / d + if gf < bf { 6.0 } else { 0.0 }) * 60.0
    } else if max == gf {
        ((bf - rf) / d + 2.0) * 60.0
    } else {
        ((rf - gf) / d + 4.0) * 60.0
    };

    (h % 360.0, s, l)
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0) / 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    if s < f32::EPSILON {
        let v = (l * 255.0).round() as u8;
        return [v, v, v];
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let channel = |mut t: f32| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        let value = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (value * 255.0).round().clamp(0.0, 255.0) as u8
    };

    [channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, RgbImage};
    use std::io::Cursor;

    #[test]
    fn hsl_round_trips() {
        for rgb in [
            [201, 162, 39],
            [47, 93, 99],
            [239, 230, 207],
            [20, 20, 20],
            [180, 40, 60],
        ] {
            let (h, s, l) = rgb_to_hsl(rgb[0], rgb[1], rgb[2]);
            let back = hsl_to_rgb(h, s, l);
            for channel in 0..3 {
                let diff = (i16::from(back[channel]) - i16::from(rgb[channel])).abs();
                assert!(diff <= 1, "{rgb:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn picks_the_colourful_half_of_the_cover() {
        let mut img = RgbImage::new(16, 8);
        for (x, _y, pixel) in img.enumerate_pixels_mut() {
            *pixel = if x < 8 {
                image::Rgb([190, 30, 30])
            } else {
                image::Rgb([128, 128, 128])
            };
        }
        let mut bytes = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();

        let palette = from_bytes(&bytes).expect("palette");
        assert!(palette.vinyl.starts_with('#'));
        assert_ne!(palette.vinyl, palette.label);
        assert_ne!(palette.vinyl, palette.dark);
    }

    #[test]
    fn grayscale_cover_has_no_palette() {
        let img = RgbImage::from_pixel(8, 8, image::Rgb([90, 90, 90]));
        let mut bytes = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        assert!(from_bytes(&bytes).is_none());
    }

    #[test]
    fn fallback_palette_is_complete() {
        let palette = Palette::fallback();
        assert_eq!(palette.vinyl.len(), 7);
        assert_eq!(palette.label.len(), 7);
    }
}
