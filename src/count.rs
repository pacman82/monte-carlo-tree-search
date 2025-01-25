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
    pub fn reward(&self, judging_player: Player) -> f32 {
        match self {
            Evaluation::Undecided(count) => count.reward(judging_player),
            Evaluation::Win(winning_player) => {
                if judging_player == *winning_player {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// A weight used to decide how much we want to explore this node.
    pub (crate) fn selection_weight(&self, total_visits_parent: f32, selecting_player: Player) -> f32 {
        match self {
            Evaluation::Undecided(count) => count.ucb(total_visits_parent, selecting_player),
            Evaluation::Win(winning_player) => {
                if selecting_player == *winning_player {
                    f32::MAX
                } else {
                    0.0
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

    pub (crate) fn into_undecided(self) -> Evaluation {
        match self {
            Evaluation::Undecided(_) => self,
            Evaluation::Win(player) => {
                let mut count = Count::default();
                count.report_win_for(player);
                Evaluation::Undecided(count)
            }
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
    pub fn total(&self) -> u32 {
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

impl AddAssign for Count {
    fn add_assign(&mut self, other: Self) {
        self.wins_player_one += other.wins_player_one;
        self.wins_player_two += other.wins_player_two;
        self.draws += other.draws;
    }
}
