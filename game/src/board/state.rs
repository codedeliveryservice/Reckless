use crate::core::color::Color;

pub struct State {
    turn: Color,
}

impl Default for State {
    fn default() -> Self {
        Self { turn: Color::White }
    }
}
