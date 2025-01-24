use std::ops::AddAssign;

use crate::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evaluation {
    Win(Player),
    /// The outcome could not be proven to be either a win, loss or draw.
    Undecided(Count),
}

impl Evaluation {

    /// A value between 0 and 1 indicating, how rewarding this outcome is for the given player. 0
    /// indicates a loss, 1 a win and 0.5 a draw. However 0.5 could also indicate an outcome which
    /// is very undecided and poses and advantage for neither player. The reward function does
    /// **not** include a term to encourge exploration. It is best used to choose a move after the
    /// tree search has been completed.
    pub fn reward(&self, player: u8) -> f32 {
        match self {
            Evaluation::Undecided(count) => count.reward(player),
            Evaluation::Win(Player::One) => {
                if player == 0 {
                    1.0
                } else {
                    0.0
                }
            }
            Evaluation::Win(Player::Two) => {
                if player == 0 {
                    0.0
                } else {
                    1.0
                }
            }
        }
    }

    /// A weight used to decide how much we want to explore this node.
    pub (crate) fn selection_weight(&self, total_visits_parent: f32, player: u8) -> f32 {
        match self {
            Evaluation::Undecided(count) => count.ucb(total_visits_parent, player),
            Evaluation::Win(Player::One) => {
                if player == 0 {
                    f32::MAX
                } else {
                    0.0
                }
            }
            Evaluation::Win(Player::Two) => {
                if player == 0 {
                    0.0
                } else {
                    f32::MAX
                }
            }
        }
    }

    /// Count of total playouts
    pub(crate) fn total(&self) -> u32 {
        match self {
            Evaluation::Undecided(count) => count.total(),
            Evaluation::Win(_) => 1,
        }
    }
}

impl Default for Evaluation {
    fn default() -> Self {
        Self::Undecided(Count::default())
    }
}

/// Counts accumulated wins, losses and draws for this part of the tree
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count {
    pub wins_player_one: u32,
    pub wins_player_two: u32,
    pub draws: u32,
}

impl Count {
    /// Assign a score of 1 for winning, 0 for loosing and 0.5 for a draw. Divided by the number of
    /// playouts. Zero playouts will result in a score of 0.5.
    pub fn reward(&self, player: u8) -> f32 {
        let total = self.total();
        if self.total() == 0 {
            return 0.5;
        }
        if player == 0 {
            (self.wins_player_one as f32 + self.draws as f32 * 0.5) / total as f32
        } else {
            debug_assert!(player == 1);
            (self.wins_player_two as f32 + self.draws as f32 * 0.5) / total as f32
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
