use rand::{seq::IndexedRandom as _, Rng};

use crate::{Evaluation, GameState, TwoPlayerGame, Ucb, UcbSolver};

/// Used to obtain an ininitial bias for the outcome of a game starting from a given board.
pub trait Bias<G: TwoPlayerGame> {
    /// The type of evaluation returned by the bias.
    type Evaluation: Evaluation;

    fn bias(&mut self, game: G, rng: &mut impl Rng) -> Self::Evaluation;

    /// Evaluation given to unexplored nodes for the purpose of choosing the best node from root.
    fn unexplored(&self) -> Self::Evaluation;

    /// Invoked then selection yields a node that has been visited before.
    fn reevaluate(&mut self, game: G, previous_evaluation: Self::Evaluation) -> Self::Evaluation;
}

pub struct RandomPlayoutUcb<G: TwoPlayerGame> {
    move_buf: Vec<G::Move>,
}

impl<G> RandomPlayoutUcb<G>
where
    G: TwoPlayerGame,
{
    pub fn new() -> Self {
        Self {
            move_buf: Vec::new(),
        }
    }
}

impl<G> Default for RandomPlayoutUcb<G>
where
    G: TwoPlayerGame,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<G> Bias<G> for RandomPlayoutUcb<G>
where
    G: TwoPlayerGame,
{
    type Evaluation = Ucb;

    fn bias(&mut self, game: G, rng: &mut impl Rng) -> Ucb {
        random_play(game, &mut self.move_buf, rng)
    }

    fn unexplored(&self) -> Ucb {
        Ucb::default()
    }

    fn reevaluate(&mut self, _game: G, previous_evaluation: Ucb) -> Self::Evaluation {
        let increment_existing = |i| if i == 0 { 0 } else { i + 1 };
        Ucb {
            wins_player_one: increment_existing(previous_evaluation.wins_player_one),
            wins_player_two: increment_existing(previous_evaluation.wins_player_two),
            draws: increment_existing(previous_evaluation.draws),
        }
    }
}

/// Obtain an initial bias by playing random moves and reporting the outcome.
pub struct RandomPlayoutUcbSolver<G: TwoPlayerGame> {
    move_buf: Vec<G::Move>,
}

impl<G: TwoPlayerGame> RandomPlayoutUcbSolver<G> {
    pub fn new() -> Self {
        Self {
            move_buf: Vec::new(),
        }
    }
}

impl<G: TwoPlayerGame> Default for RandomPlayoutUcbSolver<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G> Bias<G> for RandomPlayoutUcbSolver<G>
where
    G: TwoPlayerGame,
{
    type Evaluation = UcbSolver;

    fn bias(&mut self, game: G, rng: &mut impl Rng) -> UcbSolver {
        UcbSolver::Undecided(random_play(game, &mut self.move_buf, rng))
    }

    fn unexplored(&self) -> UcbSolver {
        UcbSolver::Undecided(Ucb::default())
    }

    fn reevaluate(&mut self, _game: G, _previous_evaluation: UcbSolver) -> UcbSolver {
        unreachable!("Solver should never visit the same leaf twice")
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
