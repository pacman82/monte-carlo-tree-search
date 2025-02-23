mod bayesian;
mod ucb;

use rand::{Rng, seq::IndexedRandom as _};

use crate::{CountWdl, CountWdlSolved, Evaluation, GameState, Player, TwoPlayerGame};

pub use self::ucb::{Ucb, UcbSolver};

/// Control selection, evaluation and backpropagation.
pub trait Explorer<G: TwoPlayerGame> {
    /// The type of evaluation returned by the bias.
    type Evaluation: Evaluation + 'static;

    /// Change propagated upwards during backpropagation.
    type Delta;

    /// Initial evaluation of a newly expanded node
    fn bias(&mut self, game: G, rng: &mut impl Rng) -> Self::Evaluation;

    /// Evaluation given to unexplored nodes for the purpose of choosing the best node from root.
    /// This evaluation is not used during selection phase. This is also the initial evaluation
    /// given to the root node, for non-terminal states.
    fn unexplored_bias(&self) -> Self::Evaluation;

    /// Invoked then selection yields a node that has been visited before.
    fn reevaluate(&mut self, game: G, evaluation: &mut Self::Evaluation) -> Self::Delta;

    /// The position of the nth child, which is selected by the explorer. Only used on nodes with no
    /// unexplored children.
    fn selected_child_pos<'a>(
        &self,
        parent_eval: &Self::Evaluation,
        child_evals: impl ExactSizeIterator<Item = &'a Self::Evaluation>,
        selecting_player: Player,
    ) -> Option<usize>;

    /// Called during backpropagation. Updates the evaluation of a node based on a propagated delta
    /// emitted by the update of a child node. In addition to that, we can also take the evaluations
    /// of the siblings of the changed child into account. The method changes the evaluation of the
    /// current node during propagation to its new value. In additon to that it emmits a delta which
    /// in turn is passed to the update of its parent node.
    fn update(
        &mut self,
        old_evaluation: &mut Self::Evaluation,
        sibling_evaluations: impl Iterator<Item = Option<Self::Evaluation>>,
        propagated_delta: Self::Delta,
        choosing_player: Player,
    ) -> Self::Delta;

    /// Initial delta for backpropagation based on the bias found for the new node.
    fn initial_delta(&self, new_evaluation: &Self::Evaluation) -> Self::Delta;

    /// Solved states will be ignored during selection phase. If there are no unsolved nodes left in
    /// the tree the search will stop.
    fn is_solved(&self, evaluation: &Self::Evaluation) -> bool;
}

pub trait CountWdlBias<G> {
    fn bias(&mut self, game: G, rng: &mut impl Rng) -> CountWdl;
}

pub trait CountWdlSolvedBias<G> {
    fn bias(&mut self, game: G, rng: &mut impl Rng) -> CountWdlSolved;
}

pub struct RandomPlayout<G: TwoPlayerGame> {
    move_buf: Vec<G::Move>,
}

impl<G> RandomPlayout<G>
where
    G: TwoPlayerGame,
{
    pub fn new() -> Self {
        Self {
            move_buf: Vec::new(),
        }
    }
}

impl<G> Default for RandomPlayout<G>
where
    G: TwoPlayerGame,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<G> CountWdlBias<G> for RandomPlayout<G>
where
    G: TwoPlayerGame,
{
    fn bias(&mut self, game: G, rng: &mut impl Rng) -> CountWdl {
        random_play(game, &mut self.move_buf, rng)
    }
}

impl<G> CountWdlSolvedBias<G> for RandomPlayout<G>
where
    G: TwoPlayerGame,
{
    fn bias(&mut self, game: G, rng: &mut impl Rng) -> CountWdlSolved {
        CountWdlSolved::Undecided(random_play(game, &mut self.move_buf, rng))
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
                };
            }
            GameState::WinPlayerTwo => {
                break CountWdl {
                    wins_player_one: 0,
                    wins_player_two: 1,
                    draws: 0,
                };
            }
            GameState::Draw => {
                break CountWdl {
                    wins_player_one: 0,
                    wins_player_two: 0,
                    draws: 1,
                };
            }
        }
    }
}
