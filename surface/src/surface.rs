use std::slice;
use std::iter::repeat;
use std::ops::{Index, IndexMut};

use super::{BOX_WIDTH, BOX_HEIGHT};
use super::{Colorspace, ColorRGBA};

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    left: usize,
    width: usize,
    top: usize,
    height: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Size {
    width: usize,
    height: usize,
}

impl Rect {
    fn overrender(&self) -> Rect {
        // TODO: remove this restriction.
        assert_eq!(self.top, 0);
        assert_eq!(self.left, 0);

        let mut width = self.width;
        let width_partial_tile = width % BOX_WIDTH;
        if width_partial_tile > 0 {
            width -= width_partial_tile;
            width += BOX_WIDTH;
        }

        let mut height = self.height;
        let height_partial_tile = height % BOX_HEIGHT;
        if height_partial_tile > 0 {
            height -= height_partial_tile;
            height += BOX_HEIGHT;
        }

        Rect {
            left: self.left,
            width: width,
            top: self.top,
            height: height,
        }
    }
}

#[derive(Clone)]
pub struct Surface<CS=ColorRGBA<u8>> {
    pub rect: Rect,
    pub align_size: Size,
    background: CS,
    buffer: Vec<CS>,
}

impl<CS> Surface<CS> where CS: Colorspace {
    pub fn iter_pixels<'a>(&'a self) -> ::std::slice::Iter<'a, CS> {
        self.buffer.iter()
    }

    pub fn iter_pixels_mut<'a>(&'a mut self) -> ::std::slice::IterMut<'a, CS> {
        self.buffer.iter_mut()
    }
}

mod zigzag {
    use super::super::{BOX_WIDTH, BOX_HEIGHT};

    pub fn to_idx(orig_size: (usize, usize), coord: (usize, usize)) -> usize {
        let (width, height) = orig_size;
        assert!(width % BOX_WIDTH == 0);
        assert!(height % BOX_HEIGHT == 0);
        let box_length = BOX_WIDTH * BOX_HEIGHT;
        let boxes_across = width / BOX_WIDTH;

        let (x, y) = coord;
        if width <= x {
            panic!("`x` out of bounds: {} <= {} < {}", 0, x, width);
        }
        if height <= y {
            panic!("`y` out of bounds: {} <= {} < {}", 0, y, height);
        }

        let box_x = x / BOX_WIDTH;
        let box_y = y / BOX_HEIGHT;
        let inner_x = x % BOX_WIDTH;
        let inner_y = y % BOX_HEIGHT;

        let mut idx = 0;
        idx += box_y * boxes_across + box_x;
        idx *= box_length;
        idx += inner_y * BOX_WIDTH + inner_x;
        idx
    }

    pub fn to_coord(orig_size: (usize, usize), idx: usize) -> (usize, usize) {
        // let (width, height) = orig_size;
        // assert!(width % BOX_WIDTH == 0);
        // assert!(height % BOX_HEIGHT == 0);
        // let box_length = BOX_WIDTH * BOX_HEIGHT;
        // let boxes_across = width / BOX_WIDTH;

        // let (box_idx, inner_idx) = (idx / box_length, idx % box_length);
        // let (box_x, box_y) = (box_idx % boxes_across, box_idx / boxes_across);
        // let (inner_x, inner_y) = (inner_idx % BOX_WIDTH, inner_idx / BOX_WIDTH);
        // (box_x * BOX_WIDTH + inner_x, box_y * BOX_HEIGHT + inner_y)
        (0, 0)
    }

}

fn align_number(number: usize, align_to: usize) -> usize {
    let mut divisions = number / align_to;
    if (number % align_to) > 0 {
        divisions += 1
    }
    return divisions * align_to;
}

impl<CS> Surface<CS> where CS: Colorspace {
    pub fn new_black(width: usize, height: usize) -> Surface<CS> {
        Surface::new(width, height, CS::black())
    }

    pub fn new(width: usize, height: usize, background: CS) -> Surface<CS> {
        let align_width = align_number(width, BOX_WIDTH);
        let align_height = align_number(height, BOX_HEIGHT);
        Surface {
            rect: Rect {
                top: 0,
                height: height,
                left: 0,
                width: width,
            },
            align_size: Size {
                width: align_width,
                height: align_height,
            },
            background: background,
            buffer: repeat(background).take(align_width * align_height).collect()
        }
    }

    pub fn divide<'a>(&'a self) -> Tiles<'a, CS> {
        Tiles::new(self)
    }

    pub fn divide_mut<'a>(&'a mut self) -> TilesMut<'a, CS> {
        TilesMut::new(self)
    }

    pub fn overrender_size(&self) -> (usize, usize) {
        (self.align_size.width, self.align_size.height)
    }

    #[inline]
    pub fn pixel_count(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.rect.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.rect.height
    }

    pub fn pixels_raw(&self) -> &[CS] {
        &self.buffer
    }

    pub fn pixels(&self) -> Vec<CS> {
        let mut out = Vec::with_capacity(self.rect.width * self.rect.height);
        for x in 0..self.rect.width {
            for y in 0..self.rect.height {
                out.push(self[(x, y)].clone());
            }
        }
        out
    }
}

impl<CS> Index<usize> for Surface<CS> where CS: Colorspace {
    type Output = CS;

    fn index<'a>(&'a self, index: usize) -> &'a CS {
        &self.buffer[index]
    }
}

impl<CS> IndexMut<usize> for Surface<CS> where CS: Colorspace {
    fn index_mut<'a>(&'a mut self, index: usize) -> &'a mut CS {
        &mut self.buffer[index]
    }
}

impl<CS> Index<(usize, usize)> for Surface<CS> where CS: Colorspace {
    type Output = CS;

    fn index<'a>(&'a self, coord: (usize, usize)) -> &'a CS {
        assert_eq!(self.rect.top, 0);
        assert_eq!(self.rect.left, 0);
        let orig_size = (self.align_size.width, self.align_size.height);
        let idx = zigzag::to_idx(orig_size, coord);
        &self.buffer[idx]
    }
}

impl<CS> IndexMut<(usize, usize)> for Surface<CS> where CS: Colorspace {
    fn index_mut<'a>(&'a mut self, coord: (usize, usize)) -> &'a mut CS {
        assert_eq!(self.rect.top, 0);
        assert_eq!(self.rect.left, 0);
        let orig_size = (self.align_size.width, self.align_size.height);
        let idx = zigzag::to_idx(orig_size, coord);
        &mut self.buffer[idx]
    }
}

pub struct Tile<'a, CS=ColorRGBA<u8>> where CS: Colorspace + 'a {
    location: Rect,
    backing: &'a [CS],
}

impl<'a, CS> Tile<'a, CS> where CS: Colorspace + 'a {
    fn new(location: Rect, backing: &'a [CS]) -> Self {
        Tile {
            location: location,
            backing: backing,
        }
    }

    fn coords(&self) -> TileCoordIter {
        TileCoordIter::new(self.location)
    }

    pub fn pixels(&'a self) -> PixelIter<'a, CS> {
        PixelIter::new(self.backing, self.coords())
    }
}

impl<'a, CS> Index<(usize, usize)> for Tile<'a, CS> where CS: Colorspace + 'a {
    type Output = CS;

    fn index<'b>(&'b self, (abs_x, abs_y): (usize, usize)) -> &'b CS {
        assert_eq!(self.location.top % BOX_HEIGHT, 0);
        assert_eq!(self.location.left % BOX_WIDTH, 0);

        let x = abs_x - self.location.left;
        if self.location.width <= x {
            panic!("`x` out of bounds: {} <= {} < {}",
                self.location.left, abs_x,
                self.location.left + self.location.width);
        }

        let y = abs_y - self.location.top;
        if self.location.height <= y {
            panic!("`y` out of bounds: {} <= {} < {}",
                self.location.top, abs_y,
                self.location.top + self.location.height);
        }
        let idx = zigzag::to_idx((BOX_WIDTH, BOX_HEIGHT), (x, y));
        &self.backing[idx]
    }
}

pub struct TileMut<'a, CS=ColorRGBA<u8>> where CS: Colorspace + 'a {
    location: Rect,
    backing: &'a mut [CS],
}

impl<'a, CS> TileMut<'a, CS> where CS: Colorspace + 'a {
    fn new(location: Rect, backing: &'a mut [CS]) -> Self {
        TileMut {
            location: location,
            backing: backing,
        }
    }

    fn coords(&self) -> TileCoordIter {
        TileCoordIter::new(self.location)
    }

    pub fn pixels(&'a self) -> PixelIter<'a, CS> {
        PixelIter::new(&self.backing, self.coords())
    }

    pub fn pixels_mut(&'a mut self) -> PixelMutIter<'a, CS> {
        let coords = self.coords();
        PixelMutIter::new(&mut self.backing, coords)
    }
}

impl<'a, CS> Index<(usize, usize)> for TileMut<'a, CS> where CS: Colorspace + 'a {
    type Output = CS;

    fn index<'b>(&'b self, (abs_x, abs_y): (usize, usize)) -> &'b CS {
        assert_eq!(self.location.top % BOX_HEIGHT, 0);
        assert_eq!(self.location.left % BOX_WIDTH, 0);

        let x = abs_x - self.location.left;
        if x < self.location.width {
            panic!("`x` out of bounds: {} <= {} < {}",
                self.location.left, abs_x,
                self.location.left + self.location.width);
        }

        let y = abs_y - self.location.top;
        if y < self.location.height {
            panic!("`y` out of bounds: {} <= {} < {}",
                self.location.top, abs_y,
                self.location.top + self.location.height);
        }

        let idx = zigzag::to_idx((BOX_WIDTH, BOX_HEIGHT), (x, y));
        &self.backing[idx]
    }
}

impl<'a, CS> IndexMut<(usize, usize)> for TileMut<'a, CS> where CS: Colorspace + 'a {
    fn index_mut<'b>(&'b mut self, coord: (usize, usize)) -> &'b mut CS {
        assert_eq!(self.location.top, 0);
        assert_eq!(self.location.left, 0);
        let orig_size = (self.location.width, self.location.height);
        let idx = zigzag::to_idx(orig_size, coord);
        &mut self.backing[idx]
    }
}


struct TileRectIter {
    size: (usize, usize),
    box_idx: usize,
    box_idx_end: usize,
}

impl TileRectIter {
    fn new(image: Rect) -> Self {
        let Rect { width, height, .. } = image.overrender();
        let idx_end = width * height / (BOX_WIDTH * BOX_HEIGHT);
        TileRectIter {
            size: (width, height),
            box_idx: 0,
            box_idx_end: idx_end,
        }
    }
}

impl Iterator for TileRectIter {
    type Item = Rect;

    fn next(&mut self) -> Option<Rect> {
        if self.box_idx_end == self.box_idx {
            return None;
        }


        let offset = self.box_idx * BOX_WIDTH * BOX_HEIGHT;
        let (x, y) = zigzag::to_coord(self.size, offset);
        self.box_idx += 1;

        let rect = Rect {
            left: x,
            width: BOX_WIDTH,
            top: y,
            height: BOX_HEIGHT,
        };
        Some(rect)
    }
}


/// Yields pixel locations inside of a local 128x8 rectangle inside of
/// a `Surface`.
struct TileCoordIter {
    /// location of the local space in global space
    rect: Rect,
    idx: usize,
    idx_end: usize,
}

impl TileCoordIter {
    fn new(rect: Rect) -> Self {
        // Ensure the subsurface is tile aligned (performance).
        assert_eq!(rect.left % BOX_WIDTH, 0);
        assert_eq!(rect.top % BOX_HEIGHT, 0);
        TileCoordIter {
            rect: rect,
            idx: 0,
            idx_end: BOX_WIDTH * BOX_HEIGHT,
        }
    }
}

impl Iterator for TileCoordIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<(usize, usize)> {
        if self.idx == self.idx_end {
            return None;
        }

        let bottom = self.rect.top + self.rect.height;
        if BOX_WIDTH * bottom <= self.idx {
            return None;
        }

        let (mut x, mut y) = (self.idx % BOX_WIDTH, self.idx / BOX_WIDTH);
        if self.rect.width <= x {
            // jump forward: (x, y) = (0, y + 1)
            self.idx += self.rect.width - x;
            x = self.idx % BOX_WIDTH;
            y = self.idx / BOX_WIDTH;
            assert_eq!(x, 0);
        }

        self.idx += 1;
        let rv = (x + self.rect.left, y + self.rect.top);
        Some(rv)
    }
}

pub struct PixelIter<'a, CS=ColorRGBA<u8>> where CS: 'a {
    items: slice::Iter<'a, CS>,
    coords: TileCoordIter,
}

impl<'a, CS> PixelIter<'a, CS> where CS: Colorspace + 'a {
    fn new(pixels: &'a [CS], coords: TileCoordIter) -> Self {
        assert_eq!(pixels.len(), BOX_HEIGHT * BOX_WIDTH);
        PixelIter { items: pixels.iter(), coords: coords }
    }
}

impl<'a, CS> Iterator for PixelIter<'a, CS> {
    type Item = (usize, usize, &'a CS);

    fn next(&mut self) -> Option<(usize, usize, &'a CS)> {
        match (self.coords.next(), self.items.next()) {
            (Some((x, y)), Some(pixel)) => Some((x, y, pixel)),
            (Some(_), None) => unreachable!(),
            (None, Some(_)) => unreachable!(),
            (None, None) => None,
        }
    }
}

pub struct PixelMutIter<'a, CS: 'a> {
    items: slice::IterMut<'a, CS>,
    coords: TileCoordIter,
}

impl<'a, CS> PixelMutIter<'a, CS> where CS: 'a {
    fn new(pixels: &'a mut [CS], coords: TileCoordIter) -> Self {
        assert_eq!(pixels.len(), BOX_HEIGHT * BOX_WIDTH);
        PixelMutIter { items: pixels.iter_mut(), coords: coords }
    }
}

impl<'a, CS> Iterator for PixelMutIter<'a, CS> {
    type Item = (usize, usize, &'a mut CS);

    fn next(&mut self) -> Option<(usize, usize, &'a mut CS)> {
        match (self.coords.next(), self.items.next()) {
            (Some((x, y)), Some(pixel)) => Some((x, y, pixel)),
            (Some(_), None) => unreachable!("coord was some"),
            (None, Some(_)) => unreachable!("items was some"),
            (None, None) => None,
        }
    }
}

pub struct Tiles<'a, CS: 'a> {
    rects: TileRectIter,
    chunks: slice::Chunks<'a, CS>,
}

impl<'a, CS> Tiles<'a, CS> where CS: Colorspace + 'a {
    pub fn new(surf: &'a Surface<CS>) -> Self {
        Tiles {
            rects: TileRectIter::new(surf.rect),
            chunks: surf.buffer.chunks(BOX_WIDTH * BOX_HEIGHT),
        }
    }
}

impl<'a, CS> Iterator for Tiles<'a, CS> where CS: Colorspace + 'a {
    type Item = Tile<'a, CS>;

    fn next(&mut self) -> Option<Tile<'a, CS>> {
        match (self.rects.next(), self.chunks.next()) {
            (Some(rect), Some(backing)) => Some(Tile::new(rect, backing)),
            (Some(_), None) => unreachable!(),
            (None, Some(_)) => unreachable!(),
            (None, None) => None,
        }
    }
}

pub struct TilesMut<'a, CS: 'a> {
    rects: TileRectIter,
    chunks: slice::ChunksMut<'a, CS>,
}

impl<'a, CS> TilesMut<'a, CS> where CS: Colorspace + 'a {
    pub fn new(surf: &'a mut Surface<CS>) -> Self {
        TilesMut {
            rects: TileRectIter::new(surf.rect),
            chunks: surf.buffer.chunks_mut(BOX_WIDTH * BOX_HEIGHT),
        }
    }
}

impl<'a, CS> Iterator for TilesMut<'a, CS> where CS: Colorspace + 'a {
    type Item = TileMut<'a, CS>;

    fn next(&mut self) -> Option<TileMut<'a, CS>> {
        match (self.rects.next(), self.chunks.next()) {
            (Some(rect), Some(backing)) => Some(TileMut::new(rect, backing)),
            (Some(_), None) => unreachable!(),
            (None, Some(_)) => unreachable!(),
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use super::{zigzag, Surface};
    use super::super::ColorRGBA;
    use test::Bencher;

    #[test]
    fn test_paint_it_red() {
        let width = 896;
        let height = 600;

        let mut surf: Surface<_> = Surface::new_black(width, height);

        {
            // let mut joiners = Vec::new();
            for tile in surf.divide_mut() {
                    let mut xtile = tile;
                    for (_, _, pixel) in xtile.pixels_mut() {
                        *pixel = ColorRGBA::new_rgb(255_u8, 0, 0)
                    }
            }
        }

        for color in surf.iter_pixels() {
            assert_eq!(color.r, 255);
            assert_eq!(color.g, 0);
            assert_eq!(color.b, 0);
        }
    }

    #[bench]
    fn bench_zigzag_to_idx(b: &mut Bencher) {
        use test::black_box;

        const WIDTH: usize = 896;
        const HEIGHT: usize = 600;
        const X: usize = 743;
        const Y: usize = 397;

        b.iter(|| {
            let width = black_box(WIDTH);
            let height = black_box(HEIGHT);
            let x = ::test::black_box(X);
            let y = ::test::black_box(Y);
            zigzag::to_idx((width, height), (x, y))
        });
    }

    #[test]
    fn test_tile_rect_iter() {
        use std::ops::Add;
        use super::{Rect, TileRectIter};

        let tile_iter = TileRectIter::new(Rect {
            left: 0,
            width: 896,
            top: 0,
            height: 600,
        });
        assert_eq!(tile_iter.map(|_| 1).fold(0_u32, Add::add), 525);
    }
}
