extern crate surface;
extern crate netpbm;

use surface::{Surface, ColorRGBA};
use std::io::{self, Write};

static SAMPLE_IMAGE: &'static [u8] = include_bytes!("../sample_image2.ppm");

const IMAGE_WIDTH: usize = 896;
const IMAGE_HEIGHT: usize = 600;
const TAKE_PIXELS: usize = 1024 * 4 + 219;

fn write_out(
	wr: &mut Write,
	surf: &Surface,
) -> io::Result<()> {
	try!(write!(wr, "P3 {} {} 255\n", surf.width(), surf.height()));
	for y in 0..surf.height() {
		for x in 0..surf.width() {
			// println!("indexing x = {}, y = {}", x, y);
			let pixel = surf[(x, y)];
			try!(write!(wr, "{} {} {}\n", pixel.r, pixel.g, pixel.b));
		}
	}
	Ok(())
}

fn main() {
	let mut rdr = io::Cursor::new(SAMPLE_IMAGE);
	let mut surf: Surface = netpbm::read_ppm(&mut rdr).unwrap();

	write!(&mut io::stderr(), "{:?}, {:?}\n", surf.rect, surf.align_size);
	for (idx, pixel) in surf.iter_pixels_mut().enumerate() {
		if (idx / 1024) % 2 == 0 {
			*pixel = ColorRGBA::new_rgb(0, 0, 0);
		}
	}
	write_out(&mut io::stdout(), &surf).unwrap();
}