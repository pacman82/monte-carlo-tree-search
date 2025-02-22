use rand::{seq::IndexedRandom as _, Rng};

use crate::{
    CountWdl, CountWdlSolved, CountWdlSolvedDelta, Evaluation, GameState, Player, TwoPlayerGame,
};

/// Control selection, evaluation and backpropagation.
pub trait Explorer<G: TwoPlayerGame> {
    /// The type of evaluation returned by the bias.
    type Evaluation: Evaluation + 'static;

    /// Change propagated upwards during backpropagation.
    type Delta;

    /// Initial evaluation of a newly expanded node
    fn bias(&mut self, game: G, rng: &mut impl Rng) -> Self::Evaluation;

    /// Evaluation given to unexplored nodes for the purpose of choosing the best node from root.
    /// This evaluation is not used during selection phase
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

    /// Initial delto for backpropagation based on the bias found for the new node.
    fn initial_delta(&self, new_evaluation: &Self::Evaluation) -> Self::Delta;

    /// Solved states will be ignored during selection phase. If there are no unsolved nodes left in
    /// the tree the search will stop.
    fn is_solved(&self, evaluation: &Self::Evaluation) -> bool;
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

impl<G> Explorer<G> for Ucb<G>
where
    G: TwoPlayerGame,
{
    type Evaluation = CountWdl;

    type Delta = CountWdl;

    fn bias(&mut self, game: G, rng: &mut impl Rng) -> CountWdl {
        random_play(game, &mut self.move_buf, rng)
    }

    fn unexplored_bias(&self) -> CountWdl {
        CountWdl::default()
    }

    fn reevaluate(&mut self, _game: G, evaluation: &mut CountWdl) -> CountWdl {
        let zero_or_one = |i| if i == 0 { 0 } else { 1 };
        let delta = CountWdl {
            wins_player_one: zero_or_one(evaluation.wins_player_one),
            wins_player_two: zero_or_one(evaluation.wins_player_two),
            draws: zero_or_one(evaluation.draws),
        };
        *evaluation += delta;
        delta
    }

    fn update(
        &mut self,
        old_evaluation: &mut Self::Evaluation,
        _sibling_evaluations_: impl Iterator<Item = Option<Self::Evaluation>>,
        propagated_delta: Self::Delta,
        _choosing_player: Player,
    ) -> Self::Delta {
        *old_evaluation += propagated_delta;
        propagated_delta
    }

    fn initial_delta(&self, new_evaluation: &Self::Evaluation) -> Self::Delta {
        *new_evaluation
    }

    fn selected_child_pos<'a>(
        &self,
        parent_eval: &CountWdl,
        child_evals: impl ExactSizeIterator<Item = &'a CountWdl>,
        selecting_player: Player,
    ) -> Option<usize> {
        child_evals
            .enumerate()
            .max_by(|&(_pos_a, eval_a), &(_pos_b, eval_b)| {
                let a = eval_a.ucb(parent_eval.total() as f32, selecting_player);
                let b = eval_b.ucb(parent_eval.total() as f32, selecting_player);
                a.partial_cmp(&b).unwrap()
            })
            .map(|(pos, _)| pos)
    }

    fn is_solved(&self, _evaluation: &Self::Evaluation) -> bool {
        false
    }
}

/// Obtain an initial bias by playing random moves and reporting the outcome.
pub struct UcbSolver<B> {
    bias: B,
}

impl<B> UcbSolver<B> {
    pub fn new() -> Self
    where
        B: Default,
    {
        Self { bias: B::default() }
    }

    pub fn with_bias(bias: B) -> Self {
        Self { bias }
    }
}

impl<B> Default for UcbSolver<B>
where
    B: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<G, B> Explorer<G> for UcbSolver<B>
where
    B: CountWdlSolvedBias<G>,
    G: TwoPlayerGame,
{
    type Evaluation = CountWdlSolved;

    type Delta = CountWdlSolvedDelta;

    fn bias(&mut self, game: G, rng: &mut impl Rng) -> CountWdlSolved {
        self.bias.bias(game, rng)
    }

    fn unexplored_bias(&self) -> CountWdlSolved {
        CountWdlSolved::default()
    }

    fn reevaluate(&mut self, _game: G, _evaluation: &mut CountWdlSolved) -> CountWdlSolvedDelta {
        unreachable!("Solver should never visit the same leaf twice")
    }

    fn update(
        &mut self,
        old_evaluation: &mut Self::Evaluation,
        sibling_evaluations: impl Iterator<Item = Option<CountWdlSolved>>,
        propagated_delta: CountWdlSolvedDelta,
        choosing_player: Player,
    ) -> CountWdlSolvedDelta {
        let previous_count = old_evaluation.into_count();
        let CountWdlSolvedDelta {
            propagated_evaluation,
            previous_count: previous_child_count,
        } = propagated_delta;
        if propagated_evaluation == CountWdlSolved::Win(choosing_player) {
            // If it is the choosing players turn, she will choose a win
            *old_evaluation = propagated_evaluation;
            return CountWdlSolvedDelta {
                propagated_evaluation,
                previous_count,
            };
        }
        // If the choosing player is not guaranteed to win let's check if there is a draw or a loss
        let loss = CountWdlSolved::Win(choosing_player.opponent());
        if propagated_evaluation.is_solved() {
            let mut acc = Some(propagated_evaluation);
            for maybe_eval in sibling_evaluations {
                let Some(child_eval) = maybe_eval else {
                    // Still has unexplored children, so we can not be sure the current node is a
                    // draw or a loss.
                    acc = None;
                    break;
                };
                if child_eval == CountWdlSolved::Draw {
                    // Found a draw, so we can be sure its not a loss
                    acc = Some(CountWdlSolved::Draw);
                } else if child_eval != loss {
                    // Found a child neither draw or loss, so we can not rule out a victory yet
                    acc = None;
                    break;
                }
            }
            if let Some(evaluation) = acc {
                *old_evaluation = evaluation;
                return CountWdlSolvedDelta {
                    propagated_evaluation: evaluation,
                    previous_count,
                };
            }
        }
        // No deterministic outcome, let's propagete the counts
        let propageted_count = match propagated_evaluation {
            CountWdlSolved::Win(Player::One) => {
                let mut count = CountWdl {
                    wins_player_one: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountWdlSolved::Win(Player::Two) => {
                let mut count = CountWdl {
                    wins_player_two: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountWdlSolved::Draw => {
                let mut count = CountWdl {
                    draws: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountWdlSolved::Undecided(count) => count,
        };

        let (new_eval, delta) = match old_evaluation {
            &mut CountWdlSolved::Undecided(mut count) => {
                count += propageted_count;
                (
                    CountWdlSolved::Undecided(count),
                    CountWdlSolvedDelta {
                        propagated_evaluation: CountWdlSolved::Undecided(propageted_count),
                        previous_count,
                    },
                )
            }
            _ => (
                *old_evaluation,
                CountWdlSolvedDelta {
                    propagated_evaluation: CountWdlSolved::Undecided(propageted_count),
                    previous_count,
                },
            ),
        };
        *old_evaluation = new_eval;
        delta
    }

    fn initial_delta(&self, new_evaluation: &Self::Evaluation) -> Self::Delta {
        CountWdlSolvedDelta {
            propagated_evaluation: *new_evaluation,
            previous_count: CountWdl::default(),
        }
    }

    fn selected_child_pos<'a>(
        &self,
        parent_eval: &Self::Evaluation,
        child_evals: impl ExactSizeIterator<Item = &'a Self::Evaluation>,
        selecting_player: Player,
    ) -> Option<usize> {
        child_evals
            .enumerate()
            .filter(|(_pos, eval)| !eval.is_solved())
            .max_by(|&(_pos_a, eval_a), &(_pos_b, eval_b)| {
                let a = eval_a
                    .undecided()
                    .unwrap()
                    .ucb(parent_eval.total() as f32, selecting_player);
                let b = eval_b
                    .undecided()
                    .unwrap()
                    .ucb(parent_eval.total() as f32, selecting_player);
                a.partial_cmp(&b).unwrap()
            })
            .map(|(pos, _)| pos)
    }

    fn is_solved(&self, evaluation: &Self::Evaluation) -> bool {
        evaluation.is_solved()
    }
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
