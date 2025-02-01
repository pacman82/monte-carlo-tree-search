use rand::{seq::IndexedRandom as _, Rng};

use crate::{Count, CountOrDecided, Evaluation, GameState, Player, TwoPlayerGame};

/// Used to obtain an ininitial bias for the outcome of a game starting from a given board.
pub trait Bias<G: TwoPlayerGame> {
    /// The type of evaluation returned by the bias.
    type Evaluation: Evaluation;

    fn bias(&mut self, game: G, move_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> Self::Evaluation;

    /// Evaluation given to unexplored nodes for the purpose of choosing the best node from root.
    fn unexplored(&self) -> Self::Evaluation;

    // init_eval_from_game_state should probably be moved to the Evaluation trait. In this scenario,
    // also think about how to handle hitting terminal states during move selection.

    /// Creating an initial evaluation for the root node, or before the first simulation. Can be
    /// used to handle terminal states.
    fn init_eval_from_game_state(&self, state: GameState<'_, G::Move>) -> Self::Evaluation;
}

/// Obtain an initial bias by playing random moves and reporting the outcome.
pub struct RandomPlayoutBias;

impl<G> Bias<G> for RandomPlayoutBias
where
    G: TwoPlayerGame,
{
    type Evaluation = CountOrDecided;

    fn bias(&mut self, game: G, move_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> CountOrDecided {
        CountOrDecided::Undecided(random_play(game, move_buf, rng))
    }

    fn init_eval_from_game_state(&self, state: GameState<'_, G::Move>) -> CountOrDecided {
        match state {
            GameState::Moves(_) => CountOrDecided::Undecided(Count::default()),
            GameState::Draw => CountOrDecided::Draw,
            GameState::WinPlayerOne => CountOrDecided::Win(Player::One),
            GameState::WinPlayerTwo => CountOrDecided::Win(Player::Two),
        }
    }

    fn unexplored(&self) -> CountOrDecided {
        CountOrDecided::Undecided(Count::default())
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
