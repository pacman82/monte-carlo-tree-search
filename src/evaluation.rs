mod count_wdl;
mod count_wdl_solved;

use std::cmp::Ordering;

use crate::{GameState, Player};

pub use self::{
    count_wdl::CountWdl,
    count_wdl_solved::{CountWdlSolved, CountWdlSolvedDelta},
};

/// Controls what information is stored for each board remembered in the nodes of the tree, how
/// to change it during backpropagation and what criteria to use to select the next node to expand.
pub trait Evaluation: Copy {
    /// Define an ordering between two evaluations, so that the greates value is the most favorable
    /// move for the given player. This method is currently used by [`crate::Tree`] in order to
    /// update the best move found so far after each playout.
    fn cmp_for(&self, other: &Self, player: Player) -> Ordering;

    /// Creating an initial evaluation for the root node, or before the first simulation. Can be
    /// used to handle terminal states.
    fn eval_for_terminal_state<M>(state: &GameState<'_, M>) -> Self;
}
