use rand::{seq::IndexedRandom as _, Rng};

use crate::{Count, Evaluation, GameState, TwoPlayerGame, Ucb};

/// Used to obtain an ininitial bias for the outcome of a game starting from a given board.
pub trait Bias<G: TwoPlayerGame> {
    /// The type of evaluation returned by the bias.
    type Evaluation: Evaluation;

    fn bias(&self, game: G, move_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> Ucb;
}

/// Obtain an initial bias by playing random moves and reporting the outcome.
pub struct RandomPlayoutBias;

impl<G> Bias<G> for RandomPlayoutBias
where
    G: TwoPlayerGame,
{
    type Evaluation = Ucb;

    fn bias(&self, game: G, move_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> Ucb {
        Ucb::Undecided(random_play(game, move_buf, rng))
    }
}

/// Play random moves, until the game is over and report the score from the perspective of the
/// player whose turn it is.
pub fn random_play<G>(mut game: G, moves_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> Count
where
    G: TwoPlayerGame,
{
    loop {
        match game.state(moves_buf) {
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
    }
}
