//https://github.com/parasyte/pixels/tree/c2454b01abc11c007d4b9de8525195af942fef0d/examples/conway

#![deny(clippy::all)]
#![forbid(unsafe_code)]

use log::{debug, error};
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit_input_helper::WinitInputHelper;

const SCREEN_WIDTH: u32 = 360;
const SCREEN_HEIGHT: u32 = 240;


pub fn run_tricolor() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let (window, p_width, p_height, mut _hidpi_factor) =
        create_window("Critters", &event_loop);

    let surface_texture = SurfaceTexture::new(p_width, p_height, &window);

    let mut life = MarGrid::new_empty(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize);
    let mut pixels = Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture)?;
    let mut paused = false;

    let mut draw_state: Option<bool> = None;


    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            life.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // For everything else, for let winit_input_helper collect events to build its state.
        // It returns `true` when it is time to update our game state and request a redraw.
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            if input.key_pressed(VirtualKeyCode::P) {
                paused = !paused;
            }
            if input.key_pressed(VirtualKeyCode::Space) {
                // Space is frame-step, so ensure we're paused
                paused = true;
            }
            if input.key_pressed(VirtualKeyCode::R) {
                life.randomize();
            }
            if input.key_pressed(VirtualKeyCode::C) {
                life.clear();
            }
            // Handle mouse. This is a bit involved since support some simple
            // line drawing (mostly because it makes nice looking patterns).
            let (mouse_cell, mouse_prev_cell) = input
                .mouse()
                .map(|(mx, my)| {
                    let (dx, dy) = input.mouse_diff();
                    let prev_x = mx - dx;
                    let prev_y = my - dy;

                    let (mx_i, my_i) = pixels
                        .window_pos_to_pixel((mx, my))
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    let (px_i, py_i) = pixels
                        .window_pos_to_pixel((prev_x, prev_y))
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    (
                        (mx_i as isize, my_i as isize),
                        (px_i as isize, py_i as isize),
                    )
                })
                .unwrap_or_default();

            if input.mouse_pressed(0) {
                debug!("Mouse click at {:?}", mouse_cell);
                draw_state = Some(life.toggle(mouse_cell.0, mouse_cell.1));
            } else if let Some(draw_alive) = draw_state {
                let release = input.mouse_released(0);
                let held = input.mouse_held(0);
                debug!("Draw at {:?} => {:?}", mouse_prev_cell, mouse_cell);
                debug!("Mouse held {:?}, release {:?}", held, release);
                // If they either released (finishing the drawing) or are still
                // in the middle of drawing, keep going.
                if release || held {
                    debug!("Draw line of {:?}", draw_alive);
                    life.set_line(
                        mouse_prev_cell.0,
                        mouse_prev_cell.1,
                        mouse_cell.0,
                        mouse_cell.1,
                        draw_alive,
                    );
                }
                // If they let go or are otherwise not clicking anymore, stop drawing.
                if release || !held {
                    debug!("Draw end");
                    draw_state = None;
                }
            }
            // Adjust high DPI factor
            if let Some(factor) = input.scale_factor_changed() {
                _hidpi_factor = factor;
            }
            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
            }
            if !paused || input.key_pressed(VirtualKeyCode::Space) {
                life.update();
            }
            window.request_redraw();
        }
    });
}


/// Generate a pseudorandom seed for the game's PRNG.
fn generate_seed() -> (u64, u64) {
    use byteorder::{ByteOrder, NativeEndian};
    use getrandom::getrandom;

    let mut seed = [0_u8; 16];

    getrandom(&mut seed).expect("failed to getrandom");

    (
        NativeEndian::read_u64(&seed[0..8]),
        NativeEndian::read_u64(&seed[8..16]),
    )
}


const INITIAL_FILL: f32 = 0.95;

#[derive(Clone, Copy, Debug, Default)]
struct Cell {
    value: u8,
}

impl Cell {
    fn new(value: u8) -> Self {
        Self { value }
    }

    #[must_use]
    fn next_state(mut self, value: u8) -> Self {
        self.value = value;
        self
    }

    fn set_alive(&mut self, value: u8) {
        *self = self.next_state(value);
    }


}

#[derive(Clone, Debug)]
struct MarGrid {
    cells: Vec<Cell>,
    scratch_cells: Vec<Cell>,
    width: usize,
    height: usize,
}

impl MarGrid {
    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        let size = width.checked_mul(height).expect("too big");
        Self {
            cells: vec![Cell::default(); size],
            scratch_cells: vec![Cell::default(); size],
            width,
            height,
        }
    }

    fn randomize(&mut self) {
        let mut rng: randomize::PCG32 = generate_seed().into();
        for c in self.cells.iter_mut() {
            let alive = randomize::f32_half_open_right(rng.next_u32()) > INITIAL_FILL;
            *c = Cell::new(alive);
        }
    }

    fn count_big_cell(&self, x: usize, y: usize) -> (usize,[usize;4]) {
        let (_, xp1) = if x == 0 {
            (self.width - 1, x + 1)
        } else if x == self.width - 1 {
            (x - 1, 0)
        } else {
            (x - 1, x + 1)
        };
        let (_, yp1) = if y == 0 {
            (self.height - 1, y + 1)
        } else if y == self.height - 1 {
            (y - 1, 0)
        } else {
            (y - 1, y + 1)
        };
        let count = self.cells[x + y *self.width].alive as usize
            + self.cells[xp1 + y * self.width].alive as usize
            + self.cells[x + yp1 * self.width].alive as usize
            + self.cells[xp1 + yp1 * self.width].alive as usize;
        // Cells in clockwise order
        let cell_pos = [x + y *self.width, 
                               xp1 + y * self.width,
                               xp1 + yp1 * self.width,
                               x + yp1 * self.width];
        (count,cell_pos)
    }

    fn update_big_cell(&mut self, n: usize, cells: [usize;4]) {
        if n == 2 {
            // No change
            return
        } else if [0,1,4].contains(&n) {
            // Invert the whole block
            for p in cells {
                self.cells[p].toggle()
            }
        } else {
            // Rotate 180 degree than invert the whole block
            let t0 = self.cells[cells[0]];
            let t1 = self.cells[cells[1]];
            let t2 = self.cells[cells[2]];
            let t3 = self.cells[cells[3]];
            self.cells[cells[0]] = t2;
            self.cells[cells[1]] = t3;
            self.cells[cells[2]] = t0;
            self.cells[cells[3]] = t1;
            for p in cells {
                self.cells[p].toggle()
            }
        }
    }

    fn update(&mut self) {
        if self.reverse {
            self.update_grid_2();
            self.update_grid_1();
        } else {
            self.update_grid_1();
            self.update_grid_2();
        }
    }

    #[inline]
    fn update_grid_1(&mut self) {
        for yt in 0..self.height/2 {
            for xt in 0..self.width/2 {
                let idx = xt*2+yt*self.width*2;
                let (x, y) = self.idx_grid(idx).unwrap();
                let (count, cell_pos) = self.count_big_cell(x,y);
                self.update_big_cell(count,cell_pos);
            }
        }
    }

    #[inline]
    fn update_grid_2(&mut self) {
        for yt in 0..self.height/2 {
            for xt in 0..self.width/2 {
                let idx = xt*2+yt*self.width*2;
                let (x, y) = self.idx_grid(idx).unwrap();
                let (count, cell_pos) = self.count_big_cell(x+1,y+1);
                self.update_big_cell(count,cell_pos);
            }
        }
    }



    fn toggle(&mut self, x: isize, y: isize) -> bool {
        if let Some(i) = self.grid_idx(x, y) {
            let was_alive = self.cells[i].alive;
            self.cells[i].set_alive(!was_alive);
            !was_alive
        } else {
            false
        }
    }

    fn clear(&mut self) {
        for c in self.cells.iter_mut() {
            *c = Cell::default();
        }
    }

    fn draw(&self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.cells.len());
        for (c, pix) in self.cells.iter().zip(screen.chunks_exact_mut(4)) {
            let color = if c.alive {
                [0xff, 0xff, 0xff, 0xff]
            } else {
                [0, 0, 0, 0xff]
            };
            pix.copy_from_slice(&color);
        }
    }

    fn set_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, alive: bool) {
        // probably should do sutherland-hodgeman if this were more serious.
        // instead just clamp the start pos, and draw until moving towards the
        // end pos takes us out of bounds.
        let x0 = x0.max(0).min(self.width as isize);
        let y0 = y0.max(0).min(self.height as isize);
        for (x, y) in line_drawing::Bresenham::new((x0, y0), (x1, y1)) {
            if let Some(i) = self.grid_idx(x, y) {
                self.cells[i].set_alive(alive);
            } else {
                break;
            }
        }
    }

    fn grid_idx<I: std::convert::TryInto<usize>>(&self, x: I, y: I) -> Option<usize> {
        if let (Ok(x), Ok(y)) = (x.try_into(), y.try_into()) {
            if x < self.width && y < self.height {
                Some(x + y * self.width)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn idx_grid<I: std::convert::TryInto<usize>>(&self, n: I) -> Option<(usize,usize)> {
        if let Ok(pos) = n.try_into() {
            let x = pos%self.width;
            let y = pos/self.width;
            Some((x,y))
        } else {
            None
        }
        
    }
}
