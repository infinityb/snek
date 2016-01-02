use std::ops::{Add, Mul, Sub};
use num::traits::{Float, ToPrimitive};
use super::{Channel, Colorspace, clamp};

pub use self::imp::ColorARGB;

#[cfg(target_endian = "little")]
mod imp {
    #[repr(C)]
    #[derive(Debug, Copy)]
    pub struct ColorARGB<T> {
        pub b: T,
        pub g: T,
        pub r: T,
        pub a: T,
    }
}

#[cfg(target_endian = "big")]
mod imp {
    #[repr(C)]
    #[derive(Debug, Copy)]
    pub struct ColorARGB<T> {
        pub a: T,
        pub r: T,
        pub g: T,
        pub b: T,
    }
}

impl<T: Clone> Clone for ColorARGB<T> {
    fn clone(&self) -> ColorARGB<T> {
        ColorARGB {
            a: self.a.clone(),
            r: self.r.clone(),
            g: self.g.clone(),
            b: self.b.clone(),

        }
    }
}

// Maybe later?: ColorARGB<f64>.quantize() -> ColorARGB<usize>
// How do we implement this more generally so that we may have ColorARGB<f64>
impl ColorARGB<f64> {
    pub fn new_rgb_clamped(r: f64, g: f64, b: f64) -> ColorARGB<u8> {
        let min_color: u8 = Channel::min_value();
        let max_color: u8 = Channel::max_value();

        ColorARGB::new_rgb(
            clamp((r * max_color as f64).round() as i32, min_color as i32, max_color as i32) as u8,
            clamp((g * max_color as f64).round() as i32, min_color as i32, max_color as i32) as u8,
            clamp((b * max_color as f64).round() as i32, min_color as i32, max_color as i32) as u8)
    }
}

impl ColorARGB<u8> {
    pub fn from_packed_argb(color: u32) -> ColorARGB<u8> {
        let a = ((color >> 24) & 0xFF) as u8;
        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >>  8) & 0xFF) as u8;
        let b = ((color >>  0) & 0xFF) as u8;
        ColorARGB { a: a, r: r, g: g, b: b }
    }

    pub fn packed(&self) -> u32 {
        let mut out = 0;
        out |= (self.a as u32) << 24;
        out |= (self.r as u32) << 16;
        out |= (self.g as u32) <<  8;
        out |= (self.b as u32) <<  0;
        out
    }
}

// Maybe later?: ColorARGB<f64>.quantize() -> ColorARGB<uint>
// How do we implement this more generally so that we may have ColorARGB<f64>
impl<T: Channel> ColorARGB<T> {
    pub fn new_argb(a: T, r: T, g: T, b: T) -> ColorARGB<T> {
        ColorARGB { a: a, r: r, g: g, b: b }
    }

    #[allow(dead_code)]
    pub fn new_rgb(r: T, g: T, b: T) -> ColorARGB<T> {
        ColorARGB::new_argb(Channel::max_value(), r, g, b)
    }

    pub fn white() -> ColorARGB<T> {
        ColorARGB::new_rgb(
            Channel::max_value(),
            Channel::max_value(),
            Channel::max_value())
    }


    pub fn black() -> ColorARGB<T> {
        ColorARGB::new_rgb(
            Channel::min_value(),
            Channel::min_value(),
            Channel::min_value())
    }

    pub fn channel_f64(&self) -> ColorARGB<f64> {
        let max_val: T = Channel::max_value();
        ColorARGB {
            r: self.r.to_f64().unwrap() / max_val.to_f64().unwrap(),
            g: self.g.to_f64().unwrap() / max_val.to_f64().unwrap(),
            b: self.b.to_f64().unwrap() / max_val.to_f64().unwrap(),
            a: self.a.to_f64().unwrap() / max_val.to_f64().unwrap(),
        }
    }
}

impl<T: Channel> Add for ColorARGB<T> {
    type Output = ColorARGB<T>;

    fn add(self, other: ColorARGB<T>) -> ColorARGB<T> {
        ColorARGB {
            r: Channel::add(self.r, other.r),
            g: Channel::add(self.g, other.g),
            b: Channel::add(self.b, other.b),
            a: Channel::add(self.a, other.a),
        }
    }
}

impl<T: Channel> Sub for ColorARGB<T> {
    type Output = ColorARGB<T>;

    fn sub(self, other: ColorARGB<T>) -> ColorARGB<T> {
        ColorARGB {
            r: Channel::sub(self.r, other.r),
            g: Channel::sub(self.g, other.g),
            b: Channel::sub(self.b, other.b),
            a: Channel::sub(self.a, other.a),
        }
    }
}

impl<T: Float> Mul for ColorARGB<T> {
    type Output = ColorARGB<T>;

    fn mul(self, other: ColorARGB<T>) -> ColorARGB<T> {
        ColorARGB {
            r: self.r * other.r,
            g: self.g * other.g,
            b: self.b * other.b,
            a: self.a * other.a
        }
    }
}

// Scalar multiplication
impl<T: Float> Mul<T> for ColorARGB<T> {
    type Output = ColorARGB<T>;

    fn mul(self, other: T) -> ColorARGB<T> {
        ColorARGB {
            r: self.r * other,
            g: self.g * other,
            b: self.b * other,
            a: self.a
        }
    }
}

impl<T> Colorspace for ColorARGB<T> where T: Channel+Copy {
    fn white() -> Self {
        ColorARGB::new_rgb(
            Channel::max_value(),
            Channel::max_value(),
            Channel::max_value())
    }

    fn black() -> Self {
        ColorARGB::new_rgb(
            Channel::min_value(),
            Channel::min_value(),
            Channel::min_value())
    }
}

#[test]
fn color_add() {
    let foo_color: ColorARGB<u8> = ColorARGB::new_argb(1, 1, 1, 1) +
            ColorARGB::new_argb(2, 2, 2, 2);
    assert_eq!(foo_color.a, 3);
    assert_eq!(foo_color.r, 3);
    assert_eq!(foo_color.g, 3);
    assert_eq!(foo_color.b, 3);

    let foo_color: ColorARGB<u8> = ColorARGB::new_argb(1, 200, 1, 1) +
        ColorARGB::new_argb(2, 200, 2, 2);
    assert_eq!(foo_color.a, 3);
    assert_eq!(foo_color.r, 255);
    assert_eq!(foo_color.g, 3);
    assert_eq!(foo_color.b, 3);
}

#[test]
fn color_sub() {
    let foo_color: ColorARGB<u8> = ColorARGB::new_argb(7, 7, 7, 7) -
            ColorARGB::new_argb(2, 2, 2, 2);
    assert_eq!(foo_color.a, 5);
    assert_eq!(foo_color.r, 5);
    assert_eq!(foo_color.g, 5);
    assert_eq!(foo_color.b, 5);
}

#[test]
fn color_mul() {
    let foo_color = ColorARGB::<f64>::new_rgb(0.5, 0.0, 0.0) * 2.0;

    assert_eq!(foo_color.a, 1.0);
    assert_eq!(foo_color.r, 1.0);
    assert_eq!(foo_color.g, 0.0);
    assert_eq!(foo_color.b, 0.0);
}

