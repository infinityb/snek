use std::cmp::{min, max, Ord};

use super::super::{ColorYUV, ColorRGB, Colorspace};
use ::Surface;

fn clamp<T: Ord>(value: T, min_value: T, max_value: T) -> T {
    max(min(value, max_value), min_value)
}

fn pixel_yuv888_to_rgb888(c: ColorYUV<u8>) -> ColorRGB<u8> {
    // TODO: is there a way to do this with integer maths?
    let (y, u, v) = (c.y as f64, c.u as f64, c.v as f64);
    let (up, vp) = (u - 128.0, v - 128.0);

    let r = (0.5 + y + 1.370705 * vp) as u32;
    let g = (0.5 + y - 0.698001 * vp - 0.337633 * up) as u32;
    let b = (0.5 + y + 1.732446 * up) as u32;

    ColorRGB {
        r: clamp(r, 0, 255) as u8,
        g: clamp(g, 0, 255) as u8,
        b: clamp(b, 0, 255) as u8,
    }
}

pub fn yuv888_to_rgb888(surf: &Surface<ColorYUV<u8>>) -> Surface<ColorRGB<u8>> {
    let mut out = Surface::new(surf.width(), surf.height(), ColorRGB::black());
    for (pin, pout) in surf.iter_pixels().zip(out.iter_pixels_mut()) {
        *pout = pixel_yuv888_to_rgb888(*pin);
    }
    out
}
