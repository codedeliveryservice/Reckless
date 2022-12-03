#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub const NUM: usize = 2;
}
