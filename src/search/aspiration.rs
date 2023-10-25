use super::Searcher;

const ASPIRATION_WINDOW: i32 = 30;
const ASPIRATION_WIDENING: i32 = 60;

impl<'a> Searcher<'a> {
    pub fn aspiration_window(&mut self, mut score: i32, depth: i32) -> i32 {
        let mut alpha = score - ASPIRATION_WINDOW;
        let mut beta = score + ASPIRATION_WINDOW;

        loop {
            score = self.alpha_beta::<true, true>(alpha, beta, depth);

            if self.stopped {
                return 0;
            }

            if alpha >= score {
                alpha -= ASPIRATION_WIDENING;
            } else if score >= beta {
                beta += ASPIRATION_WIDENING;
            } else {
                return score;
            }
        }
    }
}
