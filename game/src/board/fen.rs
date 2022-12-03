use super::Board;

pub enum ParseFenError {}

pub struct Fen;

impl Fen {
    pub(crate) fn parse(fen: &str) -> Result<Board, ParseFenError> {
        todo!()
    }
}
