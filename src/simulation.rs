use rand::{seq::SliceRandom as _, Rng};

use crate::{count::EstimatedOutcome, Count, GameState, TwoPlayerGame};

/// Play random moves, until the game is over and report the score from the perspective of the
/// player whose turn it is.
pub fn simulation(mut game: impl TwoPlayerGame, rng: &mut impl Rng) -> EstimatedOutcome {
    let mut moves_buf = Vec::new();
    let count = loop {
        match game.state(&mut moves_buf) {
            GameState::Moves(legal_moves) => {
                let selected_move = legal_moves.choose(rng).unwrap();
                game.play(selected_move)
            }
            GameState::WinPlayerOne => {
                break Count {
                    wins_player_one: 1,
                    wins_player_two: 0,
                    draws: 0,
                }
            }
            GameState::WinPlayerTwo => {
                break Count {
                    wins_player_one: 0,
                    wins_player_two: 1,
                    draws: 0,
                }
            }
            GameState::Draw => {
                break Count {
                    wins_player_one: 0,
                    wins_player_two: 0,
                    draws: 1,
                }
            }
        }
    };
    EstimatedOutcome::Undecided(count)
}
