extern crate byteorder;
extern crate tempfile;
#[macro_use]
extern crate wayland_client;
extern crate snek_engine;
extern crate mmap;

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

use mmap::{MapOption, MemoryMap};

use snek_engine::{
    Snake,
    GameState,
    Direction,
    GameObject,
};

wayland_env!(WaylandEnv,
    compositor: WlCompositor,
    shell: WlShell,
    shm: WlShm,
    seat: WlSeat
);

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

    let keybuffer = Arc::new(Mutex::new(VecDeque::new()));

    let keybuffer_evt = keybuffer.clone();
    ::std::thread::spawn(move || {
        use wayland_client::Event;
        use wayland_client::wayland::WaylandProtocolEvent as WPE;
        use wayland_client::wayland::seat::WlKeyboardEvent as KE;
        use wayland_client::wayland::seat::WlKeyboardKeyState::Pressed;

        loop {
            println!("LOOP XXXX");
            ::std::thread::sleep_ms(50);
            if let Some(Event::Wayland(event)) = evt_iter.next() {
                match event {
                    WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 103, Pressed)) => {
                        let mut presses = keybuffer_evt.lock().unwrap();
                        presses.push_back(Direction::North);
                        // println!("UP");
                    },
                    WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 105, Pressed)) => {
                        let mut presses = keybuffer_evt.lock().unwrap();
                        presses.push_back(Direction::West);
                        // println!("LEFT");
                    },
                    WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 106, Pressed)) => {
                        let mut presses = keybuffer_evt.lock().unwrap();
                        presses.push_back(Direction::East);
                        // println!("RIGHT");
                    },
                    WPE::WlKeyboard(_proxy_id, KE::Key(ser, ts, 108, Pressed)) => {
                        let mut presses = keybuffer_evt.lock().unwrap();
                        presses.push_back(Direction::South);
                        // println!("DOWN");
                    },
                    evt @ _ => {
                        // println!("{:?}", evt)
                    }
                }
            }
        }
    });

    let mut painter = GamePainter::new(shm, 512, 512, 1).unwrap();
    loop {
        run_game(&mut env.display, &surface, &mut painter, keybuffer.clone());
    }
}


fn run_game(display: &mut WlDisplay, surface: &WlSurface, painter: &mut GamePainter, keybuffer: Arc<Mutex<VecDeque<Direction>>>) {
    // let game_painter = GamePainter::new();
    let mut game_state = GameState::new(64, 64);
    loop {
        println!("LOOP 0000");
        ::std::thread::sleep_ms(100);

        let direction = {
            let mut direction = None;
            let mut presses = keybuffer.lock().unwrap();
            loop {
                if let Some(dir) = presses.pop_front() {
                    direction = Some(dir);
                } else {
                    break;
                }
            }
            direction
        };

        if let Some(dir) = direction {
            println!("PRESSED: {:?}", dir);
            game_state.set_user_direction(dir);
        }

        if let Err(err) = game_state.tick() {
            println!("Game Over: {:?}", err);
            break;
        }

        game_state.set_force_grow(false);


        let mut buffer = painter.create_buffer();
        draw_gradient(&mut buffer);

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

        surface.attach(Some(&buffer.wl_buffer), 0, 0);
        surface.damage(0, 0, 512, 512);
        surface.commit();


        display.sync_roundtrip().unwrap();
    }
}

fn draw_gradient(buffer: &mut Buffer) {
    // for pixel in buffer.memory.iter_mut() {
    //     *pixel = 0xFF000000;
    // }
    for x in 0..buffer.width {
        let mut red_val = (0x66 * x / buffer.width) as u32;
        if 0xFF < red_val {
            red_val = 0xFF;
        }
        for y in 0..buffer.height {
            let mut green_val = (0x66 * y / buffer.height) as u32;
            if 0xFF < green_val {
                green_val = 0xFF;
            }

            let mut out: u32 = 0xFF000000;
            out |= (red_val << 16);
            out |= (green_val << 8);

            buffer.memory[y * buffer.width + x] = out;
        }
    }
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
        let mut tmp = try!(tempfile::TempFile::new());

        let pixel_count = width * height;

        let mut tmp = BufWriter::new(tmp);
        for _buffer in 0..buffers {
            for _ in 0..pixel_count {
                try!(tmp.write_u32::<NativeEndian>(0xFF000000));
            }
        }
        let mut tmp = tmp.into_inner().unwrap();

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

        println!("painting buffer {:?}", offset);

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

struct SnakePainter<'a, 'b: 'a> {
    buffer: &'a mut Buffer<'b>,
}

impl<'a, 'b: 'a> SnakePainter<'a, 'b> {
    pub fn new(buffer: &'a mut Buffer<'b>) -> SnakePainter<'a, 'b> {
        SnakePainter { buffer: buffer }
    }

    pub fn paint(&mut self, snake: &Snake) {
        for (x, y) in snake.positions() {
            let x_start = x * 8;
            let x_end = (x + 1) * 8;
            let y_start = y * 8;
            let y_end = (y + 1) * 8;
            // println!("{}x{}+{}x{} from {:?}", x_start, y_start, 8, 8, (x, y));
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
        let x_start = x * 8;
        let x_end = (x + 1) * 8;
        let y_start = y * 8;
        let y_end = (y + 1) * 8;
        for x_p in x_start..x_end {
            for y_p in y_start..y_end {
                self.buffer.set_color((x_p, y_p), color);
            }
        }
    }
}
