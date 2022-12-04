use crate::core::color::Color;

pub struct State {
    pub turn: Color,
}

impl Default for State {
    fn default() -> Self {
        Self { turn: Color::White }
    }
}
