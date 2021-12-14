#[derive(Clone, Copy, Debug, Default)]
pub struct Cell {
    alive: bool,
    value: u8,
}

impl Cell {
    fn new(alive: bool, value: u8) -> Self {
        Self { alive, value }
    }

    fn set_value(&mut self, value: u8) {
        self.value = value
    }

    fn set_alive(&mut self, alive: bool) {
        self.alive = alive
    }

    fn toggle(&mut self) {
        self.alive = !self.alive
    }
}