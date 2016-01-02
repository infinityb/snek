use super::{Channel, Colorspace};

#[derive(Debug, Copy)]
pub struct ColorLA<T> {
    pub l: T,
    pub a: T,
}

impl<T: Clone> Clone for ColorLA<T> {
    fn clone(&self) -> ColorLA<T> {
        ColorLA {
            l: self.l.clone(),
            a: self.a.clone(),
        }
    }
}

impl<T: Channel> ColorLA<T> {
    pub fn new_l(l: T) -> ColorLA<T> {
        ColorLA { l: l, a: T::max_value() }
    }

    pub fn new_la(l: T, a: T) -> ColorLA<T> {
        ColorLA { l: l, a: a }
    }
}

impl<T> Colorspace for ColorLA<T> where T: Channel+Copy {
    fn white() -> Self {
        ColorLA {
            l: Channel::max_value(),
            a: Channel::max_value(),
        }
    }

    fn black() -> Self {
        ColorLA {
            l: Channel::min_value(),
            a: Channel::max_value(),
        }
    }
}