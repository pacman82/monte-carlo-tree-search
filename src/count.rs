use std::ops::AddAssign;

/// Counts accumulated wins, losses and draws for this part of the tree
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count {
    pub wins_player_one: u32,
    pub wins_player_two: u32,
    pub draws: u32,
}

impl Count {
    /// Assign a score of 1 for winning, 0 for loosing and 0.5 for a draw. Divided by the number of
    /// playouts. Zero playouts will result in a score of 0.
    pub fn reward(&self, player: u8) -> f32 {
        if player == 0 {
            (self.wins_player_one as f32 + self.draws as f32 * 0.5) / self.total() as f32
        } else {
            debug_assert!(player == 1);
            (self.wins_player_two as f32 + self.draws as f32 * 0.5) / self.total() as f32
        }
    }

    /// Upper confidence bound. Used to select which leaf to explore next. Formula balances
    /// exploration with exploitation.
    pub fn ucb(&self, total_visits_parent: f32, player: u8) -> f32 {
        self.reward(player) + (2f32 * total_visits_parent.ln() / self.total() as f32).sqrt()
    }

    /// Count of total playouts
    pub fn total(&self) -> u32 {
        self.wins_player_one + self.wins_player_two + self.draws
    }
}

impl AddAssign for Count {
    fn add_assign(&mut self, other: Self) {
        self.wins_player_one += other.wins_player_one;
        self.wins_player_two += other.wins_player_two;
        self.draws += other.draws;
    }
}
