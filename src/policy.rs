use rand::{seq::IndexedRandom as _, Rng};

use crate::{Evaluation, GameState, TwoPlayerGame, CountWdl, CountWdlSolved};

/// Control selection, evaluation and backpropagation.
pub trait Policy<G: TwoPlayerGame> {
    /// The type of evaluation returned by the bias.
    type Evaluation: Evaluation;

    /// Initial evaluation of a newly expanded node
    fn initial_evaluation(&mut self, game: G, rng: &mut impl Rng) -> Self::Evaluation;

    /// Evaluation given to unexplored nodes for the purpose of choosing the best node from root.
    /// This evaluation is not used during selection phase
    fn unexplored_bias(&self) -> Self::Evaluation;

    /// Invoked then selection yields a node that has been visited before.
    fn reevaluate(&mut self, game: G, previous_evaluation: Self::Evaluation) -> Self::Evaluation;
}

pub struct Ucb<G: TwoPlayerGame> {
    move_buf: Vec<G::Move>,
}

impl<G> Ucb<G>
where
    G: TwoPlayerGame,
{
    pub fn new() -> Self {
        Self {
            move_buf: Vec::new(),
        }
    }
}

impl<G> Default for Ucb<G>
where
    G: TwoPlayerGame,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<G> Policy<G> for Ucb<G>
where
    G: TwoPlayerGame,
{
    type Evaluation = CountWdl;

    fn initial_evaluation(&mut self, game: G, rng: &mut impl Rng) -> CountWdl {
        random_play(game, &mut self.move_buf, rng)
    }

    fn unexplored_bias(&self) -> CountWdl {
        CountWdl::default()
    }

    fn reevaluate(&mut self, _game: G, previous_evaluation: CountWdl) -> Self::Evaluation {
        let increment_existing = |i| if i == 0 { 0 } else { i + 1 };
        CountWdl {
            wins_player_one: increment_existing(previous_evaluation.wins_player_one),
            wins_player_two: increment_existing(previous_evaluation.wins_player_two),
            draws: increment_existing(previous_evaluation.draws),
        }
    }
}

/// Obtain an initial bias by playing random moves and reporting the outcome.
pub struct UcbSolver<G: TwoPlayerGame> {
    move_buf: Vec<G::Move>,
}

impl<G: TwoPlayerGame> UcbSolver<G> {
    pub fn new() -> Self {
        Self {
            move_buf: Vec::new(),
        }
    }
}

impl<G: TwoPlayerGame> Default for UcbSolver<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G> Policy<G> for UcbSolver<G>
where
    G: TwoPlayerGame,
{
    type Evaluation = CountWdlSolved;

    fn initial_evaluation(&mut self, game: G, rng: &mut impl Rng) -> CountWdlSolved {
        CountWdlSolved::Undecided(random_play(game, &mut self.move_buf, rng))
    }

    fn unexplored_bias(&self) -> CountWdlSolved {
        CountWdlSolved::Undecided(CountWdl::default())
    }

    fn reevaluate(&mut self, _game: G, _previous_evaluation: CountWdlSolved) -> CountWdlSolved {
        unreachable!("Solver should never visit the same leaf twice")
    }
}

/// Play random moves, until the game is over and report the score from the perspective of the
/// player whose turn it is.
pub fn random_play<G>(mut game: G, moves_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> CountWdl
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
                break CountWdl {
                    wins_player_one: 1,
                    wins_player_two: 0,
                    draws: 0,
                }
            }
            GameState::WinPlayerTwo => {
                break CountWdl {
                    wins_player_one: 0,
                    wins_player_two: 1,
                    draws: 0,
                }
            }
            GameState::Draw => {
                break CountWdl {
                    wins_player_one: 0,
                    wins_player_two: 0,
                    draws: 1,
                }
            }
        }
    }
}
