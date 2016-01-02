#![feature(raw, libc)]

extern crate libc;
extern crate surface;

use std::{mem, raw, slice, ptr};
use std::ffi::CStr;

use libc::{int32_t, uint32_t, c_int, size_t};

use surface::{Surface, ColorRGBA};
use surface::colorspace::Colorspace;

fn err_unwrap<T>(
    result: Result<T, uint32_t>,
    error: *mut uint32_t,
    default: T,
) -> T {
    match result {
        Ok(ok) => ok,
        Err(err) => {
            if !error.is_null() {
                unsafe { *error = err };
            }
            default
        }
    }
}

static SURFACE_ERROR_INVALID: &'static str = "INVALID";
static SURFACE_ERROR_BAD_FORMAT_NAME: &'static str = "BadFormatName";
static SURFACE_ERROR_INVALID_SIZE: &'static str = "InvalidSize";
static SURFACE_ERROR_INVALID_ARGUMENT: &'static str = "InvalidArgument";

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum SurfaceError {
    BadFormatName = 1_u32,
    InvalidSize = 2_u32,
    InvalidArgument = 3_u32,

    Invalid = 0x41414141,
}

impl SurfaceError {
     pub fn from_name(name: &[u8]) -> Result<SurfaceError, ()> {
        if name == SURFACE_ERROR_BAD_FORMAT_NAME.as_bytes() {
            return Ok(SurfaceError::BadFormatName);
        }
        if name == SURFACE_ERROR_INVALID_SIZE.as_bytes() {
            return Ok(SurfaceError::InvalidSize);
        }
        if name == SURFACE_ERROR_INVALID_ARGUMENT.as_bytes() {
            return Ok(SurfaceError::InvalidArgument);
        }
        Err(())
    }

    pub fn get_name(&self) -> &'static str {
        match *self {
            SurfaceError::Invalid => SURFACE_ERROR_INVALID,
            SurfaceError::BadFormatName => SURFACE_ERROR_BAD_FORMAT_NAME,
            SurfaceError::InvalidSize => SURFACE_ERROR_INVALID_SIZE,
            SurfaceError::InvalidArgument => SURFACE_ERROR_INVALID_ARGUMENT,
        }
    }
}

fn surface_error_from_name_helper(name: *const libc::c_char) -> Result<SurfaceError, ()> {
    if name.is_null() {
        return Err(());
    }
    let name = unsafe { CStr::from_ptr(name) };
    SurfaceError::from_name(name.to_bytes())
}

#[no_mangle]
pub extern "C" fn surface_error_from_name(name: *const libc::c_char, error: *mut uint32_t) -> SurfaceError {
    let res = surface_error_from_name_helper(name).map_err(|e| 1);
    err_unwrap(res, error, SurfaceError::Invalid)
}

#[no_mangle]
pub extern "C" fn surface_error_name(error: uint32_t) -> raw::Slice<libc::c_uchar> {
    assert!(error > 0);
    assert!(error != 0x41414141);
    let rv: SurfaceError = unsafe { mem::transmute(error) };
    unsafe { mem::transmute(rv.get_name()) }
}


// --- //

static SURFACE_FORMAT_INVALID: &'static str = "INVALID";
static SURFACE_FORMAT_RGB888: &'static str = "RGB888";
static SURFACE_FORMAT_RGBA8888: &'static str = "RGBA8888";


#[repr(u32)]
#[derive(Clone, Copy)]
pub enum SurfaceFormat {
    RGB888 = 1,
    RGBA8888 = 2,
    Invalid = 0x41414141,
}

impl SurfaceFormat {
    pub fn from_name(name: &[u8]) -> Result<SurfaceFormat, SurfaceError> {
        if name == SURFACE_FORMAT_RGB888.as_bytes() {
            return Ok(SurfaceFormat::RGB888);
        }
        if name == SURFACE_FORMAT_RGBA8888.as_bytes() {
            return Ok(SurfaceFormat::RGBA8888);
        }
        Err(SurfaceError::BadFormatName)
    }

    pub fn get_name(&self) -> &'static str {
        match *self {
            SurfaceFormat::RGB888 => SURFACE_FORMAT_RGB888,
            SurfaceFormat::RGBA8888 => SURFACE_FORMAT_RGBA8888,
            SurfaceFormat::Invalid => SURFACE_FORMAT_INVALID,
        }
    }
}

fn surface_format_from_name_helper(name: *const libc::c_char) -> Result<SurfaceFormat, SurfaceError> {
    if name.is_null() {
        return Err(SurfaceError::BadFormatName);
    }
    let name = unsafe { CStr::from_ptr(name) };
    SurfaceFormat::from_name(name.to_bytes())
}

#[no_mangle]
pub extern "C" fn surface_format_from_name(name: *const libc::c_char, error: *mut uint32_t) -> SurfaceFormat {
    let res = surface_format_from_name_helper(name).map_err(|e| e as uint32_t);
    err_unwrap(res, error, SurfaceFormat::Invalid)
}

#[no_mangle]
pub extern "C" fn surface_format_name(format: uint32_t) -> raw::Slice<libc::c_uchar> {
    assert!(format > 0);
    assert!(format != 0x41414141);
    let rv: SurfaceFormat = unsafe { mem::transmute(format) };
    unsafe { mem::transmute(rv.get_name()) }
}

// ----

#[allow(non_camel_case_types)]
pub struct surface_handle {
    surface: Surface,
}

impl Drop for surface_handle {
    fn drop(&mut self) {
        println!("dropping surface_handle");
    }
}

fn surface_new_from_buf_helper(
    width: uint32_t,
    height: uint32_t,
    input: *const u8,
    input_length: size_t,
    input_fmt: *const libc::c_char,
) -> Result<*mut surface_handle, SurfaceError> {
    if width == 0 {
        return Err(SurfaceError::InvalidSize);
    }
    if height == 0 {
        return Err(SurfaceError::InvalidSize);
    }
    if input.is_null() {
        return Err(SurfaceError::InvalidArgument)
    }
    if input_fmt.is_null() {
        return Err(SurfaceError::InvalidArgument)
    }

    let input = unsafe { slice::from_raw_parts(input, input_length as usize) };
    let format = try!(surface_format_from_name_helper(input_fmt));

    let surface = Surface::new(width as usize, height as usize, ColorRGBA::black());
    let handle = Box::new(surface_handle {
        surface: surface,
    });
    Ok(Box::into_raw(handle))
}

#[no_mangle]
pub extern "C" fn surface_new_from_buf(
    width: uint32_t,
    height: uint32_t,
    input: *const u8,
    input_length: size_t,
    input_fmt: *const libc::c_char,
    error: *mut uint32_t,
) -> *mut surface_handle {
    let result = surface_new_from_buf_helper(
            width, height, input, input_length, input_fmt)
        .map_err(|e| e as uint32_t);
    err_unwrap(result, error, ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn surface_porter_duff(
    width: uint32_t,
    height: uint32_t,
    input: *const u8,
    input_length: size_t,
    input_fmt: *const libc::c_char,
    error: *mut uint32_t,
) -> *mut surface_handle {
    let result = surface_new_from_buf_helper(
            width, height, input, input_length, input_fmt)
        .map_err(|e| e as uint32_t);
    err_unwrap(result, error, ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn surface_free(surf: *mut surface_handle) {
    let _ = unsafe { Box::from_raw(surf) };
}
