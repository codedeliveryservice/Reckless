use super::UciMessage;

/// Sends a `UciMessage` to the `GUI`.
pub fn send(message: UciMessage) {
    match message {
        UciMessage::Info => {
            println!("id name Reckless");
            println!("uciok");
        }

        UciMessage::Ready => println!("readyok"),
        UciMessage::Eval(score) => println!("evaluation {}", score),
        UciMessage::BestMove(mv) => println!("bestmove {}", mv),

        UciMessage::SearchReport {
            pv,
            depth,
            score,
            nodes,
            duration,
        } => {
            let nps = nodes as f32 / duration.as_secs_f32();
            let ms = duration.as_millis();

            print!(
                "info depth {} score cp {} nodes {} time {} nps {:.0} pv",
                depth, score, nodes, ms, nps
            );
            pv.iter().for_each(|mv| print!(" {}", mv));
            println!();
        }
    }
}
