#[derive(Clone, Copy, Debug, Default)]
impl Cell {
    fn new(alive: bool) -> Self {
        Self { alive }
    }

    #[must_use]
    fn next_state(mut self, alive: bool) -> Self {
        self.alive = alive;
        self
    }

    fn set_alive(&mut self, alive: bool) {
        *self = self.next_state(alive);
    }

    fn toggle(&mut self) {
        self.alive = !self.alive
    }
}