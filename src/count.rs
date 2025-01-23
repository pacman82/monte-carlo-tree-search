use std::ops::AddAssign;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimatedOutcome {
    WinPlayerOne,
    WinPlayerTwo,
    /// The outcome could not be proven to be either a win, loss or draw.
    Undecided(Count),
}

impl EstimatedOutcome {
    /// A value between 0 and 1 indicating, how rewarding this outcome is for the given player. 0
    /// indicates a loss, 1 a win and 0.5 a draw. However 0.5 could also indicate an outcome which
    /// is very undecided and poses and advantage for neither player. The reward function does
    /// **not** include a term to encourge exploration. It is best used to choose a move after the
    /// tree search has been completed.
    pub fn reward(&self, player: u8) -> f32 {
        match self {
            EstimatedOutcome::Undecided(count) => count.reward(player),
            EstimatedOutcome::WinPlayerOne => {
                if player == 0 {
                    1.0
                } else {
                    0.0
                }
            }
            EstimatedOutcome::WinPlayerTwo => {
                if player == 0 {
                    0.0
                } else {
                    1.0
                }
            }
        }
    }

    /// A weight used to decide how much we want to explore this node.
    pub(crate) fn selection_weight(&self, total_visits_parent: f32, player: u8) -> f32 {
        match self {
            EstimatedOutcome::Undecided(count) => count.ucb(total_visits_parent, player),
            EstimatedOutcome::WinPlayerOne => {
                if player == 0 {
                    f32::MAX
                } else {
                    0.0
                }
            }
            EstimatedOutcome::WinPlayerTwo => {
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
            EstimatedOutcome::Undecided(count) => count.total(),
            EstimatedOutcome::WinPlayerOne => 1,
            EstimatedOutcome::WinPlayerTwo => 1,
        }
    }

    pub(crate) fn propagate_child(&mut self, child: EstimatedOutcome) {
        match (self, child) {
            (EstimatedOutcome::Undecided(a), EstimatedOutcome::Undecided(b)) => {
                *a += b;
            }
            (EstimatedOutcome::WinPlayerOne, _) | (EstimatedOutcome::WinPlayerTwo, _) => (),
            (EstimatedOutcome::Undecided(count), EstimatedOutcome::WinPlayerOne) => {
                count.wins_player_one += 1;
            }
            (EstimatedOutcome::Undecided(count), EstimatedOutcome::WinPlayerTwo) => {
                count.wins_player_two += 1;
            }
        }
    }
}

impl Default for EstimatedOutcome {
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
