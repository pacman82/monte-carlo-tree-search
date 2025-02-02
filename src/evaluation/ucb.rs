use std::ops::{AddAssign, SubAssign};

use crate::Player;

/// Counts accumulated wins, losses and draws for this part of the tree
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ucb {
    pub wins_player_one: i32,
    pub wins_player_two: i32,
    pub draws: i32,
}

impl Ucb {
    /// A value between 0 and 1 indicating, how rewarding this outcome is for the given player. 0
    /// indicates a loss, 1 a win and 0.5 a draw. However 0.5 could also indicate an outcome which
    /// is very undecided and poses and advantage for neither player. The reward function does
    /// **not** include a term to encourge exploration. It is best used to choose a move after the
    /// tree search has been completed.
    pub fn reward(&self, judging_player: Player) -> f32 {
        let total = self.total();
        if self.total() == 0 {
            return 0.5;
        }
        if judging_player == Player::One {
            (self.wins_player_one as f32 + self.draws as f32 * 0.5) / total as f32
        } else {
            debug_assert!(judging_player == Player::Two);
            (self.wins_player_two as f32 + self.draws as f32 * 0.5) / total as f32
        }
    }

    /// Upper confidence bound. Used to select which leaf to explore next. Formula balances
    /// exploration with exploitation.
    pub fn ucb(&self, total_visits_parent: f32, player: Player) -> f32 {
        self.reward(player) + (2f32 * total_visits_parent.ln() / self.total() as f32).sqrt()
    }

    /// Count of total playouts
    pub fn total(&self) -> i32 {
        self.wins_player_one + self.wins_player_two + self.draws
    }

    /// Increment the count by one for the specified player.
    pub fn report_win_for(&mut self, player: Player) {
        match player {
            Player::One => self.wins_player_one += 1,
            Player::Two => self.wins_player_two += 1,
        }
    }
}

impl AddAssign for Ucb {
    fn add_assign(&mut self, other: Self) {
        self.wins_player_one += other.wins_player_one;
        self.wins_player_two += other.wins_player_two;
        self.draws += other.draws;
    }
}

impl SubAssign for Ucb {
    fn sub_assign(&mut self, other: Self) {
        self.wins_player_one -= other.wins_player_one;
        self.wins_player_two -= other.wins_player_two;
        self.draws -= other.draws;
    }
}