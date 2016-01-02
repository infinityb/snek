use super::colorspace::ColorARGB;

fn clamp<T: PartialOrd>(value: T, min_value: T, max_value: T) -> T {
    if max_value < value {
        return max_value;
    }
    if value < min_value {
        return min_value;
    }
    value
}

fn channels_float(arg: ColorARGB<u8>) -> (f64, f64, f64, f64) {
    (
        arg.a as f64 / 255.0,
        arg.r as f64 / 255.0,
        arg.g as f64 / 255.0,
        arg.b as f64 / 255.0,
    )
}

pub enum Mode {
    Over,
}

pub type Operation = fn(ColorARGB<u8>, ColorARGB<u8>) -> ColorARGB<u8>;

impl Mode {
    fn operation(&self) -> Operation {
        match *self {
            Mode::Over => porter_duff_over,
        }
    }
}

pub fn porter_duff_over(apx: ColorARGB<u8>, bpx: ColorARGB<u8>) -> ColorARGB<u8> {
    let (aal, are, agr, abl) = channels_float(apx);
    let (bal, bre, bgr, bbl) = channels_float(bpx);

    let a = aal + bal * (1.0 - aal);

    let r = (are * aal + bre * bal * (1.0 - aal)) / a;
    let g = (agr * aal + bgr * bal * (1.0 - aal)) / a;
    let b = (abl * aal + bbl * bal * (1.0 - aal)) / a;

    assert!(0.0 <= r);
    assert!(0.0 <= g);
    assert!(0.0 <= b);
    assert!(r <= 1.0);
    assert!(g <= 1.0);
    assert!(b <= 1.0);

    let a = clamp((255.0 * a) as u32, 0, 255) as u8;
    let r = clamp((255.0 * r) as u32, 0, 255) as u8;
    let g = clamp((255.0 * g) as u32, 0, 255) as u8;
    let b = clamp((255.0 * b) as u32, 0, 255) as u8;

    ColorARGB::new_argb(a, r, g, b)
}

pub unsafe fn porter_duff(tgt: &mut [u32], src: &[u32], dst: &[u32], mode: Mode) -> Result<(), &'static str> {
    use std::mem::transmute;

    if tgt.len() != src.len() {
        return Err("tgt/src len mismatch");
    }
    if dst.len() != src.len() {
        return Err("dst/src len mismatch");
    }

    let tgt: &mut [ColorARGB<u8>] = transmute(tgt);
    let src: &[ColorARGB<u8>] = transmute(src);
    let dst: &[ColorARGB<u8>] = transmute(dst);

    let op_func = mode.operation();

    for (tpx, (spx, dpx)) in tgt.iter_mut().zip(src.iter().zip(dst.iter())) {
        *tpx = op_func(*spx, *dpx);
    }

    Ok(())
}

pub unsafe fn porter_duff_inplace_dst(tgt: &mut [u32], dst: &[u32], mode: Mode) -> Result<(), &'static str> {
    use std::mem::transmute;

    if tgt.len() != dst.len() {
        return Err("tgt/dst len mismatch");
    }

    let tgt: &mut [ColorARGB<u8>] = transmute(tgt);
    let dst: &[ColorARGB<u8>] = transmute(dst);

    let op_func = mode.operation();

    for (tpx, dpx) in tgt.iter_mut().zip(dst.iter()) {
        *tpx = op_func(*tpx, *dpx);
    }

    Ok(())
}


pub unsafe fn porter_duff_inplace_src(tgt: &mut [u32], src: &[u32], mode: Mode) -> Result<(), &'static str> {
    use std::mem::transmute;

    if tgt.len() != src.len() {
        return Err("tgt/src len mismatch");
    }

    let tgt: &mut [ColorARGB<u8>] = transmute(tgt);
    let src: &[ColorARGB<u8>] = transmute(src);

    let op_func = mode.operation();

    for (tpx, spx) in tgt.iter_mut().zip(src.iter()) {
        *tpx = op_func(*spx, *tpx);
    }

    Ok(())
}


