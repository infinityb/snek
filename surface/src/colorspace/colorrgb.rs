use super::{Colorspace, Channel};

#[derive(Debug, Copy)]
pub struct ColorRGB<T> {
    pub r: T,
    pub g: T,
    pub b: T,
}

impl<T: Clone> Clone for ColorRGB<T> {
    fn clone(&self) -> ColorRGB<T> {
        ColorRGB {
            r: self.r.clone(),
            g: self.g.clone(),
            b: self.b.clone(),
        }
    }
}

impl<T: Channel> ColorRGB<T> {
    #[allow(dead_code)]
    pub fn new_rgb(r: T, g: T, b: T) -> ColorRGB<T> {
        ColorRGB { r: r, g: g, b: b }
    }
}

impl<T> Colorspace for ColorRGB<T> where T: Channel+Copy {
    fn white() -> Self {
        ColorRGB::new_rgb(
            Channel::max_value(),
            Channel::max_value(),
            Channel::max_value())
    }

    fn black() -> Self {
        ColorRGB::new_rgb(
            Channel::min_value(),
            Channel::min_value(),
            Channel::min_value())
    }
}