use std::ops::{Add, Mul, Sub};
use num::traits::{Float, ToPrimitive};
use super::{Channel, Colorspace, clamp};

#[derive(Debug, Copy)]
pub struct ColorRGBA<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

impl<T: Clone> Clone for ColorRGBA<T> {
    fn clone(&self) -> ColorRGBA<T> {
        ColorRGBA {
            r: self.r.clone(),
            g: self.g.clone(),
            b: self.b.clone(),
            a: self.a.clone()
        }
    }
}

// Maybe later?: ColorRGBA<f64>.quantize() -> ColorRGBA<usize>
// How do we implement this more generally so that we may have ColorRGBA<f64>
impl ColorRGBA<f64> {
    pub fn new_rgb_clamped(r: f64, g: f64, b: f64) -> ColorRGBA<u8> {
        let min_color: u8 = Channel::min_value();
        let max_color: u8 = Channel::max_value();

        ColorRGBA::new_rgb(
            clamp((r * max_color as f64).round() as i32, min_color as i32, max_color as i32) as u8,
            clamp((g * max_color as f64).round() as i32, min_color as i32, max_color as i32) as u8,
            clamp((b * max_color as f64).round() as i32, min_color as i32, max_color as i32) as u8)
    }
}

impl ColorRGBA<u8> {
    pub fn from_packed_rgba(color: u32) -> ColorRGBA<u8> {
        let r = ((color >> 24) & 0xFF) as u8;
        let g = ((color >> 16) & 0xFF) as u8;
        let b = ((color >>  8) & 0xFF) as u8;
        let a = ((color >>  0) & 0xFF) as u8;
        ColorRGBA { r: r, g: g, b: b, a: a }
    }
}

// Maybe later?: ColorRGBA<f64>.quantize() -> ColorRGBA<uint>
// How do we implement this more generally so that we may have ColorRGBA<f64>
impl<T: Channel> ColorRGBA<T> {
    pub fn new_rgba(r: T, g: T, b: T, a: T) -> ColorRGBA<T> {
        ColorRGBA { r: r, g: g, b: b, a: a }
    }

    #[allow(dead_code)]
    pub fn new_rgb(r: T, g: T, b: T) -> ColorRGBA<T> {
        ColorRGBA { r: r, g: g, b: b, a: Channel::max_value() }
    }

    pub fn channel_f64(&self) -> ColorRGBA<f64> {
        let max_val: T = Channel::max_value();
        ColorRGBA {
            r: self.r.to_f64().unwrap() / max_val.to_f64().unwrap(),
            g: self.g.to_f64().unwrap() / max_val.to_f64().unwrap(),
            b: self.b.to_f64().unwrap() / max_val.to_f64().unwrap(),
            a: self.a.to_f64().unwrap() / max_val.to_f64().unwrap(),
        }
    }
}

impl<T: Channel> Add for ColorRGBA<T> {
    type Output = ColorRGBA<T>;

    fn add(self, other: ColorRGBA<T>) -> ColorRGBA<T> {
        ColorRGBA {
            r: Channel::add(self.r, other.r),
            g: Channel::add(self.g, other.g),
            b: Channel::add(self.b, other.b),
            a: Channel::add(self.a, other.a),
        }
    }
}

impl<T: Channel> Sub for ColorRGBA<T> {
    type Output = ColorRGBA<T>;

    fn sub(self, other: ColorRGBA<T>) -> ColorRGBA<T> {
        ColorRGBA {
            r: Channel::sub(self.r, other.r),
            g: Channel::sub(self.g, other.g),
            b: Channel::sub(self.b, other.b),
            a: Channel::sub(self.a, other.a),
        }
    }
}

impl<T: Float> Mul for ColorRGBA<T> {
    type Output = ColorRGBA<T>;

    fn mul(self, other: ColorRGBA<T>) -> ColorRGBA<T> {
        ColorRGBA {
            r: self.r * other.r,
            g: self.g * other.g,
            b: self.b * other.b,
            a: self.a * other.a
        }
    }
}

// Scalar multiplication
impl<T: Float> Mul<T> for ColorRGBA<T> {
    type Output = ColorRGBA<T>;

    fn mul(self, other: T) -> ColorRGBA<T> {
        ColorRGBA {
            r: self.r * other,
            g: self.g * other,
            b: self.b * other,
            a: self.a
        }
    }
}

impl<T> Colorspace for ColorRGBA<T> where T: Channel+Copy {
    fn white() -> Self {
        ColorRGBA::new_rgb(
            Channel::max_value(),
            Channel::max_value(),
            Channel::max_value())
    }

    fn black() -> Self {
        ColorRGBA::new_rgb(
            Channel::min_value(),
            Channel::min_value(),
            Channel::min_value())
    }
}

#[test]
fn color_add() {
    let foo_color: ColorRGBA<u8> = ColorRGBA::new_rgba(1, 1, 1, 1) +
            ColorRGBA::new_rgba(2, 2, 2, 2);
    assert_eq!(foo_color.r, 3);
    assert_eq!(foo_color.g, 3);
    assert_eq!(foo_color.b, 3);
    assert_eq!(foo_color.a, 3);

    let foo_color: ColorRGBA<u8> = ColorRGBA::new_rgba(200, 1, 1, 1) +
        ColorRGBA::new_rgba(200, 2, 2, 2);
    assert_eq!(foo_color.r, 255);
    assert_eq!(foo_color.g, 3);
    assert_eq!(foo_color.b, 3);
    assert_eq!(foo_color.a, 3);
}

#[test]
fn color_sub() {
    let foo_color: ColorRGBA<u8> = ColorRGBA::new_rgba(7, 7, 7, 7) -
            ColorRGBA::new_rgba(2, 2, 2, 2);
    assert_eq!(foo_color.r, 5);
    assert_eq!(foo_color.g, 5);
    assert_eq!(foo_color.b, 5);
    assert_eq!(foo_color.a, 5);
}

#[test]
fn color_mul() {
    let foo_color = ColorRGBA::<f64>::new_rgb(0.5, 0.0, 0.0) * 2.0;

    assert_eq!(foo_color.r, 1.0);
    assert_eq!(foo_color.g, 0.0);
    assert_eq!(foo_color.b, 0.0);
    assert_eq!(foo_color.a, 1.0);
}

