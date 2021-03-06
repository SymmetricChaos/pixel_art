#![deny(clippy::all)]
#![forbid(unsafe_code)]

use log::{debug, error};
use pixels::{Error, Pixels, SurfaceTexture};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit_input_helper::WinitInputHelper;

use crate::auxiliary::randomizer::generate_seed;
use crate::auxiliary::window::{create_window, SCREEN_WIDTH, SCREEN_HEIGHT};


// We are going to create a very simple sandpile dynamical system
// https://en.wikipedia.org/wiki/Abelian_sandpile_model

// height at which a pile topples, not really changeable
const TOPPLE_HEIGHT: u32 = 4;

// proportion of cells to fill when randomizing
const RANDOM_FILL: f32 = 0.07; 

// how many grains the huge center pile gets
const CENTER_HEIGHT: u32 = 98304;

// how many grains a clicked pixel is set to
const CLICK_HEIGHT: u32 = 256;



pub fn run_piles() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let (window, p_width, p_height, mut _hidpi_factor) =
        create_window(
            "Sandpiles", 
            &event_loop);
    
    let surface_texture = SurfaceTexture::new(p_width, p_height, &window);

    let mut piles = SandPiles::new_center(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize);
    let mut pixels = Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture)?;
    let mut paused = false;

    let mut draw_state: Option<bool> = None;

    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            piles.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
            }
        }

        // For everything else, for let winit_input_helper collect events to build its state.
        // It returns `true` when it is time to update our game state and request a redraw.
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
            }
            if input.key_pressed(VirtualKeyCode::P) {
                paused = !paused;
            }
            if input.key_pressed(VirtualKeyCode::Space) {
                // Space is frame-step, so ensure we're paused
                paused = true;
            }
            if input.key_pressed(VirtualKeyCode::R) {
                piles.clear();
                piles.randomize();
            }
            if input.key_pressed(VirtualKeyCode::N) {
                piles.clear();
                piles.center_pile();
            }
            if input.key_pressed(VirtualKeyCode::L) {
                piles.clear();
                piles.center_line();
            }
            if input.key_pressed(VirtualKeyCode::C) {
                piles.clear();
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
                draw_state = Some(piles.set_pile(mouse_cell.0, mouse_cell.1));
            } else if let Some(draw_alive) = draw_state {
                let release = input.mouse_released(0);
                let held = input.mouse_held(0);
                debug!("Draw at {:?} => {:?}", mouse_prev_cell, mouse_cell);
                debug!("Mouse held {:?}, release {:?}", held, release);
                // If they either released (finishing the drawing) or are still
                // in the middle of drawing, keep going.
                if release || held {
                    debug!("Draw line of {:?}", draw_alive);
                    piles.set_line(
                        mouse_prev_cell.0,
                        mouse_prev_cell.1,
                        mouse_cell.0,
                        mouse_cell.1
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
                piles.update();
            }
            window.request_redraw();
        }
    });
}



#[derive(Clone, Copy, Debug, Default)]
struct Pile {
    grains: u32,
}

impl Pile {
    fn new(grains: u32) -> Self {
        Self { grains }
    }

    #[must_use]
    fn next_state(mut self) -> Self {
        if self.grains >= TOPPLE_HEIGHT {
            self.grains -= TOPPLE_HEIGHT;
        }
        self
    }

    fn give_grain(&self) -> u32 {
        if self.grains >= TOPPLE_HEIGHT {
            1
        } else {
            0
        }
    }

    fn add_grains(&self, grains: u32) -> Self {
        Pile::new(self.grains.saturating_add(grains))
    }

    fn set_grains_inplace(&mut self, grains: u32) {
        self.grains = grains
    }

}


fn pixel_color(height: u32) -> [u8; 4] {
    if height > TOPPLE_HEIGHT {
        [0xff, 0xff, 0, 0xff]
    } else if height == TOPPLE_HEIGHT {
        [0, 0xdd, 0xdd, 0xff]
    } else {
        [(height as u8)*50, 0, (height as u8)*80, 0xff]
    }
}

#[derive(Clone, Debug)]
struct SandPiles {
    piles: Vec<Pile>,
    width: usize,
    height: usize,
    scratch_piles: Vec<Pile>,
}

impl SandPiles {
    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        let size = width.checked_mul(height).expect("too big");
        Self {
            piles: vec![Pile::default(); size],
            scratch_piles: vec![Pile::default(); size],
            width,
            height,
        }
    }

    fn new_center(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.center_pile();
        result
    }

    fn center_pile(&mut self) {
        let pos = self.grid_idx(SCREEN_WIDTH/2, SCREEN_HEIGHT/2).unwrap();
        self.piles[pos].set_grains_inplace(CENTER_HEIGHT);
    }

    fn center_line(&mut self) {
        let y = SCREEN_HEIGHT/2;
        for x in 0..SCREEN_WIDTH {
            if x > 40 && x < SCREEN_WIDTH-40 {
                let pos = self.grid_idx(x, y).unwrap();
                self.piles[pos].set_grains_inplace(512);
            }
        }
    }

    fn randomize(&mut self) {
        let mut rng: randomize::PCG32 = generate_seed().into();
        for c in self.piles.iter_mut() {
            let alive = randomize::f32_half_open_right(rng.next_u32()) < RANDOM_FILL;
            if alive {
                let grains = rng.next_u32() % 64;
                *c = Pile::new(grains);
            }
        }
    }

    fn clear(&mut self) {
        for c in self.piles.iter_mut() {
            *c = Pile::default();
        }
    }

    // Each neighbor tall enough to topple contributes a single grain
    fn count_tall_neibs(&self, x: usize, y: usize) -> u32 {
        let (xm1, xp1) = if x == 0 {
            (self.width - 1, x + 1)
        } else if x == self.width - 1 {
            (x - 1, 0)
        } else {
            (x - 1, x + 1)
        };
        let (ym1, yp1) = if y == 0 {
            (self.height - 1, y + 1)
        } else if y == self.height - 1 {
            (y - 1, 0)
        } else {
            (y - 1, y + 1)
        };
        self.piles[x + ym1 * self.width].give_grain()
            + self.piles[xm1 + y * self.width].give_grain()
            + self.piles[xp1 + y * self.width].give_grain()
            + self.piles[x + yp1 * self.width].give_grain()
    }

    fn update(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let neibs = self.count_tall_neibs(x, y);
                let idx = x + y * self.width;
                let next = self.piles[idx].next_state().add_grains(neibs);
                // Write into `self.scratch_piles`, since we're still reading from `self.piles`
                self.scratch_piles[idx] = next;
            }
        }
        // We've been writing to a the temporary scratch_piles
        // Now that we're done just swap the memory
        std::mem::swap(&mut self.scratch_piles, &mut self.piles);
    }

    fn draw(&self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.piles.len());
        for (c, pix) in self.piles.iter().zip(screen.chunks_exact_mut(4)) {
            let color = pixel_color(c.grains);
            pix.copy_from_slice(&color);
        }
    }

    fn set_pile(&mut self, x: isize, y: isize) -> bool {
        if let Some(i) = self.grid_idx(x, y) {
            self.piles[i].set_grains_inplace(CLICK_HEIGHT);
        }
        true
    }

    fn set_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize) {
        // probably should do sutherland-hodgeman if this were more serious.
        // instead just clamp the start pos, and draw until moving towards the
        // end pos takes us out of bounds.
        let x0 = x0.max(0).min(self.width as isize);
        let y0 = y0.max(0).min(self.height as isize);
        for (x, y) in line_drawing::Bresenham::new((x0, y0), (x1, y1)) {
            if let Some(i) = self.grid_idx(x, y) {
                self.piles[i].set_grains_inplace(CLICK_HEIGHT);
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
