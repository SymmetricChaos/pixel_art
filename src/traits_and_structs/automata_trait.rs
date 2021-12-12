pub trait CellAutomata {
    fn draw(&self, screen: &mut [u8]);
    fn set_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, alive: bool);
    fn toggle(&mut self);
    fn invert(&mut self);
    fn clear(&mut self);
    fn describe() -> String;
}