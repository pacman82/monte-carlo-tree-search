use rand::{seq::IndexedRandom as _, Rng};

use crate::{Ucb, UcbSolver, Evaluation, GameState, TwoPlayerGame};

/// Used to obtain an ininitial bias for the outcome of a game starting from a given board.
pub trait Bias<G: TwoPlayerGame> {
    /// The type of evaluation returned by the bias.
    type Evaluation: Evaluation;

    fn bias(&mut self, game: G, move_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> Self::Evaluation;

    /// Evaluation given to unexplored nodes for the purpose of choosing the best node from root.
    fn unexplored(&self) -> Self::Evaluation;
}

pub struct RandomPlayoutUcb;

impl<G> Bias<G> for RandomPlayoutUcb where G: TwoPlayerGame {
    type Evaluation = Ucb;

    fn bias(&mut self, game: G, move_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> Ucb {
        random_play(game, move_buf, rng)
    }

    fn unexplored(&self) -> Ucb {
        Ucb::default()
    }
}

/// Obtain an initial bias by playing random moves and reporting the outcome.
pub struct RandomPlayoutUcbSolver;

impl<G> Bias<G> for RandomPlayoutUcbSolver
where
    G: TwoPlayerGame,
{
    type Evaluation = UcbSolver;

    fn bias(&mut self, game: G, move_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> UcbSolver {
        UcbSolver::Undecided(random_play(game, move_buf, rng))
    }

    fn unexplored(&self) -> UcbSolver {
        UcbSolver::Undecided(Ucb::default())
    }
}

/// Play random moves, until the game is over and report the score from the perspective of the
/// player whose turn it is.
pub fn random_play<G>(mut game: G, moves_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> Ucb
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
                break Ucb {
                    wins_player_one: 1,
                    wins_player_two: 0,
                    draws: 0,
                }
            }
            GameState::WinPlayerTwo => {
                break Ucb {
                    wins_player_one: 0,
                    wins_player_two: 1,
                    draws: 0,
                }
            }
            GameState::Draw => {
                break Ucb {
                    wins_player_one: 0,
                    wins_player_two: 0,
                    draws: 1,
                }
            }
        }
    }
}
