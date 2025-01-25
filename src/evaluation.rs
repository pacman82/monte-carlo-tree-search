use std::ops::AddAssign;

use crate::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evaluation {
    Win(Player),
    Draw,
    /// The outcome could not yet be proven to be either a win, loss or draw.
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
            Evaluation::Draw => 0.5,
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
    pub(crate) fn selection_weight(
        &self,
        total_visits_parent: f32,
        selecting_player: Player,
    ) -> f32 {
        match self {
            Evaluation::Undecided(count) => count.ucb(total_visits_parent, selecting_player),
            Evaluation::Draw => 0.5,
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
            Evaluation::Win(_) | Evaluation::Draw => 1,
        }
    }

    /// Convert solved solutions to their underterministic counter part
    pub(crate) fn into_count(self) -> Count {
        match self {
            Evaluation::Undecided(count) => count,
            Evaluation::Draw => Count {
                draws: 1,
                ..Count::default()
            },
            Evaluation::Win(player) => {
                let mut count = Count::default();
                count.report_win_for(player);
                count
            }
        }
    }

    /// `true` if the board evaluating to `self` is **proven** to be better or at least as good as
    /// other for the given player.
    pub fn strictly_not_worse_for(&self, other: &Evaluation, player: Player) -> bool {
        match (self, other) {
            // We can not do better than a win for the judging player
            (Evaluation::Win(s), Evaluation::Win(o)) => {
                if player == *s {
                    true
                } else {
                    *o != player
                }
            }
            (Evaluation::Win(s), Evaluation::Undecided(_))
            | (Evaluation::Win(s), Evaluation::Draw) => *s == player,
            (Evaluation::Draw, Evaluation::Win(o))
            | (Evaluation::Undecided(_), Evaluation::Win(o)) => *o != player,
            (Evaluation::Draw, Evaluation::Draw) => true,
            (Evaluation::Draw, Evaluation::Undecided(_))
            | (Evaluation::Undecided(_), Evaluation::Undecided(_))
            | (Evaluation::Undecided(_), Evaluation::Draw) => false,
        }
    }

    pub fn is_solved(&self) -> bool {
        match self {
            Evaluation::Win(_) | Evaluation::Draw => true,
            Evaluation::Undecided(_) => false,
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

#[cfg(test)]
mod test {
    use crate::{Count, Evaluation, Player};

    #[test]
    fn strictly_not_worse_for_examples() {
        assert!(Evaluation::Win(Player::One)
            .strictly_not_worse_for(&Evaluation::Win(Player::One), Player::One));
        assert!(Evaluation::Win(Player::One)
            .strictly_not_worse_for(&Evaluation::Win(Player::One), Player::Two));
        assert!(!Evaluation::Win(Player::Two)
            .strictly_not_worse_for(&Evaluation::Win(Player::One), Player::One));
        assert!(Evaluation::Win(Player::One)
            .strictly_not_worse_for(&Evaluation::Win(Player::Two), Player::One));
        assert!(Evaluation::Win(Player::One).strictly_not_worse_for(&Evaluation::Draw, Player::One));
        assert!(
            !Evaluation::Win(Player::Two).strictly_not_worse_for(&Evaluation::Draw, Player::One)
        );
        assert!(
            !Evaluation::Draw.strictly_not_worse_for(&Evaluation::Win(Player::One), Player::One)
        );
        assert!(Evaluation::Draw.strictly_not_worse_for(&Evaluation::Win(Player::Two), Player::One));
        assert!(Evaluation::Draw.strictly_not_worse_for(&Evaluation::Draw, Player::One));
        assert!(!Evaluation::Draw
            .strictly_not_worse_for(&Evaluation::Undecided(Count::default()), Player::One));
        assert!(!Evaluation::Undecided(Count::default())
            .strictly_not_worse_for(&Evaluation::Win(Player::One), Player::One));
        assert!(Evaluation::Undecided(Count::default())
            .strictly_not_worse_for(&Evaluation::Win(Player::Two), Player::One));
        assert!(!Evaluation::Undecided(Count::default())
            .strictly_not_worse_for(&Evaluation::Undecided(Count::default()), Player::One));
        assert!(!Evaluation::Undecided(Count::default())
            .strictly_not_worse_for(&Evaluation::Draw, Player::One));
    }
}
