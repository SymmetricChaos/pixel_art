// Adapted from the Pixels example with Conway's Game of Life

#![deny(clippy::all)]
#![forbid(unsafe_code)]

use log::{debug, error};
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit_input_helper::WinitInputHelper;

// Small window dimensions that should scale nicely on a 1080p or 4K screen
const SCREEN_WIDTH: u32 = 360;
const SCREEN_HEIGHT: u32 = 240;

// We are going to create a very simple sandpile dynamical system
// https://en.wikipedia.org/wiki/Abelian_sandpile_model

// This isn't really changeable but is lets us avoid a magic number
const TOPPLE_HEIGHT: u32 = 4;
const RANDOM_FILL: f32 = 0.95; // proportion of cells left empty when randomly filling
const CENTER_HEIGHT: u32 = 65536; // how many grains the huge center pile gets



fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let (window, p_width, p_height, mut _hidpi_factor) =
        create_window("Sandpile System", &event_loop);

    let surface_texture = SurfaceTexture::new(p_width, p_height, &window);

    let mut piles = SandPiles::new_random(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize);
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
                piles.clear();
                piles.randomize();
            }
            if input.key_pressed(VirtualKeyCode::N) {
                piles.clear();
                piles.center_pile();
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


// COPYPASTE: ideally this could be shared.

/// Create a window for the game.
///
/// Automatically scales the window to cover about 2/3 of the monitor height.
///
/// # Returns
///
/// Tuple of `(window, surface, width, height, hidpi_factor)`
/// `width` and `height` are in `PhysicalSize` units.
fn create_window(
    title: &str,
    event_loop: &EventLoop<()>,
) -> (winit::window::Window, u32, u32, f64) {
    // Create a hidden window so we can estimate a good default window size
    let window = winit::window::WindowBuilder::new()
        .with_visible(false)
        .with_title(title)
        .build(event_loop)
        .unwrap();
    let hidpi_factor = window.scale_factor();

    // Get dimensions
    let width = SCREEN_WIDTH as f64;
    let height = SCREEN_HEIGHT as f64;
    let (monitor_width, monitor_height) = {
        if let Some(monitor) = window.current_monitor() {
            let size = monitor.size().to_logical(hidpi_factor);
            (size.width, size.height)
        } else {
            (width, height)
        }
    };
    let scale = (monitor_height / height * 2.0 / 3.0).round().max(1.0);

    // Resize, center, and display the window
    let min_size: winit::dpi::LogicalSize<f64> =
        PhysicalSize::new(width, height).to_logical(hidpi_factor);
    let default_size = LogicalSize::new(width * scale, height * scale);
    let center = LogicalPosition::new(
        (monitor_width - width * scale) / 2.0,
        (monitor_height - height * scale) / 2.0,
    );
    window.set_inner_size(default_size);
    window.set_min_inner_size(Some(min_size));
    window.set_outer_position(center);
    window.set_visible(true);

    let size = default_size.to_physical::<f64>(hidpi_factor);

    (
        window,
        size.width.round() as u32,
        size.height.round() as u32,
        hidpi_factor,
    )
}

#[derive(Clone, Copy, Debug, Default)]
struct Pile {
    height: u32,
}

impl Pile {
    fn new(height: u32) -> Self {
        Self { height }
    }

    #[must_use]
    fn next_state(mut self) -> Self {
        if self.height >= TOPPLE_HEIGHT {
            self.height -= TOPPLE_HEIGHT;
        }
        self
    }

    fn give_grain(&self) -> u32 {
        if self.height >= TOPPLE_HEIGHT {
            1
        } else {
            0
        }
    }

    fn add_grains(&self, grains: u32) -> Self {
        Pile::new(self.height.saturating_add(grains))
    }

    fn set_grains_inplace(&mut self, grains: u32) {
        self.height = grains
    }

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

fn pixel_color(height: u32) -> [u8; 4] {
    if height > TOPPLE_HEIGHT {
        [0xff, 0xff, 0, 0xff]
    } else if height == TOPPLE_HEIGHT {
        [0, 0xdd, 0xdd, 0xff]
    } else {
        [(height as u8)*63, 0, (height as u8)*63, 0xff]
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

    fn new_random(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.randomize();
        result
    }

    fn randomize(&mut self) {
        let mut rng: randomize::PCG32 = generate_seed().into();
        for c in self.piles.iter_mut() {
            let alive = randomize::f32_half_open_right(rng.next_u32()) > RANDOM_FILL;
            if alive {
                let grains = rng.next_u32() % 64;
                *c = Pile::new(grains);
            }
        }
    }

    fn center_pile(&mut self) {
        let pos = self.grid_idx(SCREEN_WIDTH/2, SCREEN_HEIGHT/2).unwrap();
        self.piles[pos].set_grains_inplace(CENTER_HEIGHT);
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
                // Write into scratch_piles, since we're still reading from `self.piles`
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
            let color = pixel_color(c.height);
            pix.copy_from_slice(&color);
        }
    }

    fn set_pile(&mut self, x: isize, y: isize) -> bool {
        if let Some(i) = self.grid_idx(x, y) {
            self.piles[i].set_grains_inplace(256);
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
                self.piles[i].set_grains_inplace(256);
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
