extern crate surface;
extern crate netpbm;

use surface::{Surface, ColorRGBA};
use std::io::{self, Write};

const IMAGE_WIDTH: usize = 896;
const IMAGE_HEIGHT: usize = 600;
const TAKE_PIXELS: usize = 1024 * 4 + 219;

fn write_out(
	wr: &mut Write,
	width: usize, height: usize,
	pixels: &[ColorRGBA<u8>],
) -> io::Result<()> {
	// try!(write!(wr, "P3 {} {} 255\n", width, height));
	for pixel in pixels.iter() {
		// try!(write!(wr, "{} {} {}\n", pixel.r, pixel.g, pixel.b));
	}
	Ok(())
}

fn main() {
	let mut surf: Surface<_> = Surface::new_black(IMAGE_WIDTH, IMAGE_HEIGHT);

	let mut idx = 0;
	for tile in surf.divide_mut() {
		let mut xtile = tile;
        for (_, _, pixel) in xtile.pixels_mut() {
            *pixel = if idx < TAKE_PIXELS {
                ColorRGBA::new_rgb(255_u8, 0, 0)
            } else {
                ColorRGBA::new_rgb(0, 0, 0)
            };
            idx += 1;
        }
	}
	write_out(&mut io::stdout(), IMAGE_WIDTH, IMAGE_HEIGHT, &surf.pixels()).unwrap();
}