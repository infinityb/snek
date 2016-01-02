#![feature(raw)]

extern crate byteorder;
extern crate tempfile;
#[macro_use]
extern crate wayland_client;
extern crate snek_engine;
extern crate mmap;
extern crate time;
extern crate surface;

use byteorder::{WriteBytesExt, NativeEndian};

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::io::{self, Seek, SeekFrom, Write, BufWriter};
use std::os::unix::io::AsRawFd;

use wayland_client::Proxy;
use wayland_client::wayland::{get_display, WlDisplay};
use wayland_client::wayland::compositor::{WlSurface, WlCompositor};
use wayland_client::wayland::shell::WlShell;
use wayland_client::wayland::shm::{WlShm, WlShmPool, WlShmFormat, WlBuffer};
use wayland_client::wayland::data_device::WlDataDeviceManager;
use wayland_client::wayland::seat::WlSeat;
use wayland_client::EventIterator;

use surface::Surface;
use surface::colorspace::ColorARGB;
use surface::compositing::{
    porter_duff,
    porter_duff_inplace_dst,
    porter_duff_inplace_src,
    Mode as PorterDuffMode,
};

use mmap::{MapOption, MemoryMap};

use snek_engine::{
    Snake,
    GameState,
    Direction,
    GameObject,
    SnakePositions,
};

wayland_env!(WaylandEnv,
    compositor: WlCompositor,
    shell: WlShell,
    shm: WlShm,
    seat: WlSeat
);

#[derive(Copy, Clone)]
enum KeyboardEvent {
    Up,
    Down,
    Left,
    Right,
    Space,
    G, H,
}

impl KeyboardEvent {
    pub fn priority(&self) -> u16 {
        match *self {
            KeyboardEvent::Up => 1,
            KeyboardEvent::Down => 1,
            KeyboardEvent::Left => 1,
            KeyboardEvent::Right => 1,
            KeyboardEvent::Space => 2,
            KeyboardEvent::G => 0,
            KeyboardEvent::H => 0,
        }
    }
}

fn replace_state(evt: &mut Option<KeyboardEvent>, new_event: KeyboardEvent) {
    if let Some(prev_evt) = *evt {
        if prev_evt.priority() <= new_event.priority() {
            *evt = Some(new_event);
        }
    } else {
        *evt = Some(new_event);
    }
}

fn flush_keyboard_buf(evt_iter: &mut EventIterator) -> Option<KeyboardEvent> {
    use wayland_client::Event;
    use wayland_client::wayland::WaylandProtocolEvent as WPE;
    use wayland_client::wayland::seat::WlKeyboardEvent as KE;
    use wayland_client::wayland::seat::WlKeyboardKeyState::Pressed;

    let mut out = None;
    loop {
        if let Some(Event::Wayland(event)) = evt_iter.next() {
            match event {
                WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 103, Pressed)) => {
                    replace_state(&mut out, KeyboardEvent::Up);
                },
                WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 105, Pressed)) => {
                    replace_state(&mut out, KeyboardEvent::Left);
                },
                WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 106, Pressed)) => {
                    replace_state(&mut out, KeyboardEvent::Right);
                },
                WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 108, Pressed)) => {
                    replace_state(&mut out, KeyboardEvent::Down);
                },
                WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 57, Pressed)) => {
                    replace_state(&mut out, KeyboardEvent::Space);
                },
                WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 34, Pressed)) => {
                    replace_state(&mut out, KeyboardEvent::G);
                },
                WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 35, Pressed)) => {
                    replace_state(&mut out, KeyboardEvent::H);
                },
                evt @ _ => {
                    println!("{:?}", evt)
                }
            }
        } else { break }
    }
    out
}

fn main() {
    let display = match get_display() {
        Some(d) => d,
        None => panic!("Unable to connect to a wayland compositor.")
    };

    // Use wayland_env! macro to get the globals and an event iterator
    let (mut env, mut evt_iter) = WaylandEnv::init(display);

    // Get shortcuts to the globals.
    // Here we only use the version 1 of the interface, so no checks are needed.
    let compositor = env.compositor.as_ref().map(|o| &o.0).unwrap();

    let shell = env.shell.as_ref().map(|o| &o.0).unwrap();
    let shm = env.shm.as_ref().map(|o| &o.0).unwrap();
    let seat = env.seat.as_ref().map(|o| &o.0).unwrap();

    let mut keyboard = seat.get_keyboard();
    keyboard.set_evt_iterator(&evt_iter);

    let surface = compositor.create_surface();
    let shell_surface = shell.get_shell_surface(&surface);

    // make our surface as a toplevel one
    shell_surface.set_toplevel();

    let mut painter = GamePainter::new(shm, 512, 512, 3).unwrap();
    loop {
        run_game(&mut env.display, &surface, &mut painter, &mut evt_iter);
    }
}


fn run_game(display: &mut WlDisplay, surface: &WlSurface, painter: &mut GamePainter, evt_iter: &mut EventIterator) {
    use std::cmp::{min, max};
    use time::{SteadyTime, Duration as TimeDuration};
    use std::time::Duration;

    const FRAME_NANOS: i64 = 2 * 16_666_666;
    const TICK_NANOS: i64 = 100_000_000;

    let background = get_background_surface();
    let frame_duration = TimeDuration::nanoseconds(FRAME_NANOS);
    let tick_duration = TimeDuration::nanoseconds(TICK_NANOS);
    let mut next_frame = SteadyTime::now();
    let mut next_tick = SteadyTime::now();
    let mut paused = false;

    let mut bg_xoff: isize = 0;
    let mut bg_xoff_dir: isize = 4;
    let mut bg_yoff: isize = 0;
    let mut bg_yoff_dir: isize = 1;

    // let game_painter = GamePainter::new();
    let mut game_state = GameState::new(64, 64);
    loop {
        let mut sleep_dur = TimeDuration::seconds(1);
        let now = SteadyTime::now();

        let mut emit_frame = false;
        while next_frame <= now {
            emit_frame = true;
            next_frame = next_frame + frame_duration;
        }
        if emit_frame {
            let sleep_next = next_frame - now;
            if TimeDuration::zero() < sleep_next {
                sleep_dur = min(sleep_dur, sleep_next);
            }
        }

        let mut emit_tick = false;
        while next_tick <= now {
            emit_tick = true;
            next_tick = next_tick + tick_duration;
        }
        if emit_tick {
            let sleep_next = next_tick - now;
            if TimeDuration::zero() < sleep_next {
                sleep_dur = min(sleep_dur, sleep_next);
            }
        }

        if emit_tick {
            let keyboard_event = flush_keyboard_buf(evt_iter);

            let mut direction = None;
            match keyboard_event {
                Some(KeyboardEvent::Up) => direction = Some(Direction::North),
                Some(KeyboardEvent::Down) => direction = Some(Direction::South),
                Some(KeyboardEvent::Left) => direction = Some(Direction::West),
                Some(KeyboardEvent::Right) => direction = Some(Direction::East),
                Some(KeyboardEvent::Space) => paused = !paused,
                Some(KeyboardEvent::G) => game_state.set_force_grow(true),
                Some(KeyboardEvent::H) => game_state.set_force_grow(false),
                None => (),
            };

            if let Some(dir) = direction {
                game_state.set_user_direction(dir);
            }
        }



        if emit_tick && !paused {
            if let Err(err) = game_state.tick() {
                println!("Game Over: {:?}", err);
                break;
            }
        }

        if emit_frame {
            if !paused || true {
                bg_xoff += bg_xoff_dir;
                bg_yoff += bg_yoff_dir;
                if bg_xoff == 512 || bg_xoff == 0 {
                    bg_xoff_dir = -bg_xoff_dir;
                }
                if bg_yoff == 512 || bg_yoff == 0 {
                    bg_yoff_dir = -bg_yoff_dir;
                }
            }

            let mut buffer = painter.create_buffer();
            draw_background(&background, bg_xoff as usize, bg_yoff as usize, &mut buffer);

            {
                let mut painter = SnakePainter::new(&mut buffer);
                painter.paint(game_state.get_snake());
            }
            {
                let mut painter = ObjectPainter::new(&mut buffer);
                for (pos, object) in game_state.object_iter() {
                    painter.paint(pos, object);
                }
            }

            if paused {
                draw_paused_screen(&mut buffer);
            }

            surface.attach(Some(&buffer.wl_buffer), 0, 0);
            surface.damage(0, 0, 512, 512);
            surface.commit();

            display.sync_roundtrip().unwrap();
        }

        if TimeDuration::zero() < sleep_dur {
            ::std::thread::sleep_ms(1 + sleep_dur.num_milliseconds() as u32);
        }
    }
}

fn u8_slice_to_u32_slice(inp: &[u8]) -> &[u32] {
    use std::raw::Slice;
    use std::mem::transmute;

    let mut slice: Slice<u8> = unsafe { transmute(inp) };
    assert_eq!(slice.len % 4, 0);
    slice.len = slice.len / 4;
    unsafe { transmute(slice) }
}

fn get_background_surface() -> Surface<ColorARGB<u8>> {
    let background_px = u8_slice_to_u32_slice(include_bytes!("../background.bin"));
    let mut background: Surface<ColorARGB<u8>> = Surface::new(1024, 1024, ColorARGB::black());

    let mut bg_px_iter = background_px.iter();

    for (px, idx) in bg_px_iter.zip(0..512*512) {
        let (y, x) = (idx / 512 * 2, idx % 512 * 2);
        background[(x, y)] = ColorARGB::from_packed_argb(*px);
        background[(x, y+1)] = ColorARGB::from_packed_argb(*px);
        background[(x+1, y)] = ColorARGB::from_packed_argb(*px);
        background[(x+1, y+1)] = ColorARGB::from_packed_argb(*px);
    }

    background
}

fn draw_background(background: &Surface<ColorARGB<u8>>, xoff: usize, yoff: usize, buffer: &mut Buffer) {
    let mut buffer_mem = buffer.memory.iter_mut();
    for y in 0..512 {
        for x in 0..512 {
            let to_px = buffer_mem.next().unwrap();
            *to_px = background[(x+xoff, y+yoff)].packed();
        }
    }
}

fn draw_paused_screen(buffer: &mut Buffer) {
    let pause = u8_slice_to_u32_slice(include_bytes!("../pause.bin"));
    unsafe {
        porter_duff_inplace_src(&mut buffer.memory, pause, PorterDuffMode::Over)
    }.unwrap();

}

struct GamePainter {
    width: usize,
    height: usize,
    buffers: usize,
    buffer_pixel_count: usize,

    backing: MemoryMap,
    pool: WlShmPool,
    ctr: usize,
}

impl GamePainter {
    pub fn new(shm: &WlShm, width: usize, height: usize, buffers: usize) -> io::Result<GamePainter> {
        // create a tempfile to write on
        let tmp = try!(tempfile::TempFile::new());

        let pixel_count = width * height;

        let mut tmp = BufWriter::new(tmp);
        for _buffer in 0..buffers {
            for _ in 0..pixel_count {
                try!(tmp.write_u32::<NativeEndian>(0xFF000000));
            }
        }
        let tmp = tmp.into_inner().unwrap();

        let backing = try!(MemoryMap::new(4 * pixel_count * buffers, &[
            MapOption::MapNonStandardFlags(0x01),
            MapOption::MapReadable,
            MapOption::MapWritable,
            MapOption::MapFd(tmp.as_raw_fd()),
        ]).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{:?}", e))));

        let pool = shm.create_pool(tmp.as_raw_fd(), (4 * pixel_count * buffers) as i32);

        let mut painter = GamePainter {
            width: width,
            height: height,
            buffers: buffers,
            buffer_pixel_count: pixel_count,

            backing: backing,
            pool: pool,
            ctr: 0,
        };
        painter.init_frames();

        Ok(painter)
    }

    fn get_memory(&mut self, buffer_idx: usize) -> Result<&mut [u32], ()> {
        if self.buffers <= buffer_idx {
            return Err(());
        }

        let (data, length) = (self.backing.data() as *mut u32, self.backing.len() / 4);
        let mapped_slice = unsafe { std::slice::from_raw_parts_mut(data, length) };

        let s_idx = self.buffer_pixel_count * buffer_idx;
        Ok(&mut mapped_slice[s_idx..][..self.buffer_pixel_count])
    }

    pub fn init_frames(&mut self) {
        let this = [0xFF880000, 0xFF008800, 0xFF000088];

        for i in 0..self.buffers {
            let buffer = self.get_memory(i).unwrap();
            for pixel in buffer.iter_mut() {
                *pixel = this[i % 3];
            }
        }
    }

    fn create_buffer(&mut self) -> Buffer {
        let width = self.width as i32;
        let height = self.height as i32;

        let offset = self.ctr % self.buffers;
        self.ctr += 1;

        let buffer = self.pool.create_buffer(
            (4 * self.width * self.height * offset) as i32,
            width, height, 4 * width,
            WlShmFormat::Argb8888 as u32);

        Buffer {
            width: self.width,
            height: self.height,
            memory: self.get_memory(offset).unwrap(),
            wl_buffer: buffer,
        }
    }

    // pub fn paint(&mut self, &GameState) -> WlBuffer {
    //     let mut buffer = self.create_buffer();
    // }
}

struct Buffer<'a> {
    width: usize,
    height: usize,
    memory: &'a mut [u32],
    wl_buffer: WlBuffer,
}

impl<'a> Buffer<'a> {
    pub fn set_color(&mut self, (x, y): (usize, usize), color: u32) {
        self.memory[y * self.width + x] = color;
    }
}

enum SnakeJoint {
    Endpoint((usize, usize)),
    Joint(((usize, usize), (usize, usize))),
}

struct SnakeJointer<'a> {
    positions: SnakePositions<'a>,
    previous: Option<(usize, usize)>,
}

impl<'a> SnakeJointer<'a> {
    fn new(pos: SnakePositions<'a>) -> SnakeJointer<'a> {
        SnakeJointer {
            positions: pos,
            previous: None,
        }
    }
}

impl<'a> Iterator for SnakeJointer<'a> {
    type Item = SnakeJoint;

    fn next(&mut self) -> Option<SnakeJoint> {
        match (self.positions.next(), self.previous.take()) {
            (None, None) => None,
            (None, Some(ppos)) => Some(SnakeJoint::Endpoint(ppos)),
            (Some(new_pos), None) => {
                self.previous = Some(new_pos);
                Some(SnakeJoint::Endpoint(new_pos))
            }
            (Some(new_pos), Some(ppos)) => {
                self.previous = Some(new_pos);
                Some(SnakeJoint::Joint((new_pos, ppos)))
            }
        }
    }
}

struct SnakePainter<'a, 'b: 'a> {
    buffer: &'a mut Buffer<'b>,
}

impl<'a, 'b: 'a> SnakePainter<'a, 'b> {
    pub fn new(buffer: &'a mut Buffer<'b>) -> SnakePainter<'a, 'b> {
        SnakePainter { buffer: buffer }
    }

    pub fn paint(&mut self, snake: &Snake) {
        use std::cmp::{min, max};

        for part in SnakeJointer::new(snake.positions()) {
            let (x0, y0, x1, y1) = match part {
                SnakeJoint::Endpoint((x, y)) => (x, y, x, y),
                SnakeJoint::Joint(((x0, y0), (x1, y1))) => {
                    (min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1))
                },
            };

            let x_start = x0 * 8 + 1;
            let x_end = (x1 + 1) * 8 - 1;
            let y_start = y0 * 8 + 1;
            let y_end = (y1 + 1) * 8 - 1;

            for x_p in x_start..x_end {
                for y_p in y_start..y_end {
                    self.buffer.set_color((x_p, y_p), 0xFFFFFFFF);
                }
            }
        }
    }
}

struct ObjectPainter<'a, 'b: 'a> {
    buffer: &'a mut Buffer<'b>,
}

impl<'a, 'b: 'a> ObjectPainter<'a, 'b> {
    pub fn new(buffer: &'a mut Buffer<'b>) -> ObjectPainter<'a, 'b> {
        ObjectPainter { buffer: buffer }
    }

    pub fn paint(&mut self, (x, y): (usize, usize), obj: &GameObject) {
        let color = match *obj {
            GameObject::Food => 0xFFFF0000,
            GameObject::Wall => 0xFFFF00FF,
        };
        let x_start = x * 8 + 1;
        let x_end = (x + 1) * 8 - 1;
        let y_start = y * 8 + 1;
        let y_end = (y + 1) * 8 - 1;
        for x_p in x_start..x_end {
            for y_p in y_start..y_end {
                self.buffer.set_color((x_p, y_p), color);
            }
        }
    }
}
