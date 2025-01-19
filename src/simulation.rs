use rand::{seq::SliceRandom as _, Rng};

use crate::{Count, GameState, TwoPlayerGame};

/// Play random moves, until the game is over and report the score from the perspective of the
/// player whose turn it is.
pub fn simulation(mut game: impl TwoPlayerGame, rng: &mut impl Rng) -> Count {
    let start_player = game.current_player();
    let mut moves_buf = Vec::new();
    loop {
        match game.state(&mut moves_buf) {
            GameState::Moves(legal_moves) => {
                let selected_move = legal_moves.choose(rng).unwrap();
                game.play(selected_move)
            }
            GameState::Win => {
                break Count {
                    wins_current_player: (start_player == game.current_player()) as u32,
                    wins_other_player: (start_player != game.current_player()) as u32,
                    draws: 0,
                }
            }
            GameState::Loss => {
                break Count {
                    wins_current_player: (start_player != game.current_player()) as u32,
                    wins_other_player: (start_player == game.current_player()) as u32,
                    draws: 0,
                }
            }
            GameState::Draw => {
                break Count {
                    wins_current_player: 0,
                    wins_other_player: 0,
                    draws: 1,
                }
            }
        }
    }
}