
#![deny(clippy::all)]
#![forbid(unsafe_code)]

use log::{debug, error};
use pixels::{Error, Pixels, SurfaceTexture};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit_input_helper::WinitInputHelper;

use crate::auxiliary::randomizer::generate_seed;
use crate::auxiliary::window::{create_window, SCREEN_WIDTH, SCREEN_HEIGHT};


pub fn run_elementary() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let (window, p_width, p_height, mut _hidpi_factor) =
        create_window(
            "Rule 110", 
            &event_loop);
    
    let surface_texture = SurfaceTexture::new(p_width, p_height, &window);

    let mut automata = Rule110::new_random(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize);
    let mut pixels = Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture)?;
    let mut paused = false;

    let mut draw_state: Option<bool> = None;

    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            automata.draw(pixels.get_frame());
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
                match paused {
                    true => println!("paused"),
                    false => println!("unpaused"),
                }
            }
            if input.key_pressed(VirtualKeyCode::Space) {
                // Space is frame-step, so ensure we're paused
                println!("frame advanced");
                paused = true;
            }
            if input.key_pressed(VirtualKeyCode::R) {
                println!("reset with random coditions");
                automata.clear();
                automata.randomize();
            }
            if input.key_pressed(VirtualKeyCode::C) {
                println!("screen cleared and active line reset");
                automata.clear();
                automata.active_line = 1;
            }
            if input.key_pressed(VirtualKeyCode::N) {
                println!("active line reset");
                automata.active_line = 1;
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
                draw_state = Some(automata.toggle(mouse_cell.0, mouse_cell.1));
            } else if let Some(draw_alive) = draw_state {
                let release = input.mouse_released(0);
                let held = input.mouse_held(0);
                debug!("Draw at {:?} => {:?}", mouse_prev_cell, mouse_cell);
                debug!("Mouse held {:?}, release {:?}", held, release);
                // If they either released (finishing the drawing) or are still
                // in the middle of drawing, keep going.
                if release || held {
                    debug!("Draw line of {:?}", draw_alive);
                    automata.set_line(
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
                automata.update();
            }
            window.request_redraw();
        }
    });
}


const INITIAL_FILL: f32 = 0.5;

#[derive(Clone, Copy, Debug, Default)]
struct Cell {
    alive: bool
}

impl Cell {
    fn new(alive: bool) -> Self {
        Self { alive }
    }

    fn next_state(self, neibs: (bool,bool,bool)) -> Self {
        let alive = match neibs {
            (true, true, true)    => false,
            (true, true, false)   => true,
            (true, false, true)   => true,
            (true, false, false)  => false,
            (false, true, true)   => true,
            (false, true, false)  => true,
            (false, false, true)  => true,
            (false, false, false) => false,
        };
        Self::new(alive)
    }

    fn toggle(&mut self) {
        self.alive = !self.alive
    }

    fn set_alive(&mut self) {
        self.alive = true
    }

    fn _set_dead(&mut self) {
        self.alive = false
    }

}

#[derive(Clone, Debug)]
struct Rule110 {
    cells: Vec<Cell>,
    width: usize,
    height: usize,
    active_line: usize,
}

impl Rule110 {
    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        let size = width.checked_mul(height).expect("too big");
        Self {
            cells: vec![Cell::default(); size],
            width,
            height,
            active_line: 1,
        }
    }

    fn new_random(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.randomize();
        result
    }

    fn randomize(&mut self) {
        // Randomize the first row
        let mut rng: randomize::PCG32 = generate_seed().into();
        for (n, c) in self.cells.iter_mut().enumerate() {
            if n as u32 > SCREEN_WIDTH {
                break
            }
            let alive = randomize::f32_half_open_right(rng.next_u32()) > INITIAL_FILL;
            *c = Cell::new(alive);
        }
    }

    fn neibs(&self, x: usize, y: usize) -> (bool,bool,bool) {
        let (xm1, xp1) = if x == 0 {
            (self.width - 1, x + 1)
        } else if x == self.width - 1 {
            (x - 1, 0)
        } else {
            (x - 1, x + 1)
        };
        let (ym1, _) = if y == 0 {
            (self.height - 1, y + 1)
        } else if y == self.height - 1 {
            (y - 1, 0)
        } else {
            (y - 1, y + 1)
        };
        (self.cells[xm1 + ym1 * self.width].alive,
            self.cells[x + ym1 * self.width].alive,
            self.cells[xp1 + ym1 * self.width].alive)
    }

    fn update(&mut self) {
        let y = self.active_line;

        if y == 0 {
            // Do nothing
        } else {
            for x in 0..self.width {
                let neibs = self.neibs(x, y);
                let idx = x + y * self.width;
                let next = self.cells[idx].next_state(neibs);
                // Write into `self.scratch_cells`, since we're still reading from `self.cells`
                self.cells[idx] = next;
            }
            self.active_line += 1;
            if self.active_line >= self.height {
                self.active_line = 0
            }
            //std::mem::swap(&mut self.scratch_cells, &mut self.cells);
        }
    }

    fn clear(&mut self) {
        self.active_line = 1;
        for c in self.cells.iter_mut() {
            *c = Cell::default();
        }
    }

    fn toggle(&mut self, x: isize, y: isize) -> bool {
        if let Some(i) = self.grid_idx(x, y) {
            let was_alive = self.cells[i].alive;
            self.cells[i].toggle();
            !was_alive
        } else {
            false
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

    fn set_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, _alive: bool) {
        // probably should do sutherland-hodgeman if this were more serious.
        // instead just clamp the start pos, and draw until moving towards the
        // end pos takes us out of bounds.
        let x0 = x0.max(0).min(self.width as isize);
        let y0 = y0.max(0).min(self.height as isize);
        for (x, y) in line_drawing::Bresenham::new((x0, y0), (x1, y1)) {
            if let Some(i) = self.grid_idx(x, y) {
                self.cells[i].set_alive();
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
}
