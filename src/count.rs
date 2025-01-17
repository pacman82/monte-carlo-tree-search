use std::{mem, ops::AddAssign};

/// Counts accumulated wins, losses and draws for this part of the tree
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count {
    /// Number of wins for the player who is about to make the next turn
    pub wins_current_player: u32,
    /// Number of wins for the other player, who is waiting for his/her turn
    pub wins_other_player: u32,
    /// Number of draws
    pub draws: u32,
}

impl Count {
    /// Assign a score of 1 for winning, 0 for loosing and 0.5 for a draw. Divided by the number of
    /// playouts. Zero playouts will result in a score of 0.
    pub fn score(&self) -> f32 {
        (self.wins_current_player as f32 + self.draws as f32 * 0.5)
            / self.total() as f32
    }

    /// The score from the other players perspective. Useful during backpropagation, if we want to
    /// evaluate the score from the perspective of the parent node, and therefore from the
    /// perspecitive of the other player.
    pub fn flip_players(&mut self) {
        mem::swap(&mut self.wins_current_player, &mut self.wins_other_player);
    }

    /// Upper confidence bound. Used to select which leaf to explore next. Formula balances
    /// exploration with exploitation.
    pub fn ucb(&self, total_visits_parent: f32) -> f32 {
        self.score() + (2f32 * total_visits_parent.ln() / self.total() as f32).sqrt()
    }

    /// Count of total playouts
    pub fn total(&self) -> u32 {
        self.wins_current_player + self.wins_other_player + self.draws
    }
}

impl AddAssign for Count {
    fn add_assign(&mut self, other: Self) {
        self.wins_current_player += other.wins_current_player;
        self.wins_other_player += other.wins_other_player;
        self.draws += other.draws;
    }
}