mod count_wdl;
mod count_wdl_solved;

use std::cmp::Ordering;

use crate::{GameState, Player};

pub use self::{
    count_wdl::CountWdl,
    count_wdl_solved::{CountWdlSolvedDelta, CountWdlSolved},
};

/// Controls what information is stored for each board remembered in the nodes of the tree, how
/// to change it during backpropagation and what criteria to use to select the next node to expand.
pub trait Evaluation: Copy {
    /// Define an ordering between two evaluations, so that the greates value is the most favorable
    /// move for the given player. This method is currently used by [`crate::Tree`] in order to
    /// update the best move found so far after each playout.
    fn cmp_for(&self, other: &Self, player: Player) -> Ordering;

    /// A weight used to decide how much we want to explore this node, compared to its siblings.
    /// Higher weightns make a node more likely to be selected.
    fn selection_weight(&self, parent_eval: &Self, selecting_player: Player) -> f32;

    /// Solved states will be ignored during selection phase. If there are no unsolved nodes left
    /// in the tree the search will stop.
    fn is_solved(&self) -> bool;

    /// Creating an initial evaluation for the root node, or before the first simulation. Can be
    /// used to handle terminal states.
    fn init_from_game_state<M>(state: &GameState<'_, M>) -> Self;
}
