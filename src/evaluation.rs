mod count;
mod ucb_solver;

use std::cmp::Ordering;

use crate::{GameState, Player};

pub use self::{count::Count, ucb_solver::{UcbSolver, CountOrDecidedDelta}};

/// Controls what information is stored for each board remembered in the nodes of the tree, how
/// to change it during backpropagation and what criteria to use to select the next node to expand.
pub trait Evaluation: Copy {
    /// Used during backpropagation to pass information from a child node to its parent.
    type Delta;

    /// Define an ordering between two evaluations, so that the greates value is the most favorable
    /// move for the given player. This method is currently used by [`crate::Tree`] in order to
    /// update the best move found so far after each playout.
    fn cmp_for(&self, other: &Self, player: Player) -> Ordering;

    /// A weight used to decide how much we want to explore this node, compared to its siblings.
    /// Higher weightns make a node more likely to be selected.
    fn selection_weight(&self, parent_eval: &Self, selecting_player: Player) -> f32;

    /// Called during backpropagation. Updates the evaluation of a node based on a propagated delta
    /// emitted by the update of a child node. In addition to that, we can also take the evaluations
    /// of the siblings of the changed child into account. The method changes the evaluation of the
    /// current node during propagation to its new value. In additon to that it emmits a delta which
    /// in turn is passed to the update of its parent node.
    fn update(
        &mut self,
        sibling_evaluations: impl Iterator<Item = Option<Self>>,
        propagated_delta: Self::Delta,
        choosing_player: Player,
    ) -> Self::Delta;

    /// Solved states will be ignored during selection phase. If there are no unsolved nodes left
    /// in the tree the search will stop.
    fn is_solved(&self) -> bool;

    /// Initial delto for backpropagation based on the bias found for the new node.
    fn initial_delta(&self) -> Self::Delta;

    /// Creating an initial evaluation for the root node, or before the first simulation. Can be
    /// used to handle terminal states.
    fn init_from_game_state<M>(state: &GameState<'_, M>) -> Self;
}
