use std::cmp::Ordering;

use crate::{GameState, Player};

use super::{CountWdl, Evaluation};

/// Use an Upper Confidence Bound to select the next node to expand. In addition to the use of the
/// "classic" upper confidence bound, this evaluation also features variants for states such as
/// `Draw` and `Win`. This allows the tree search to proof the outcome of a game and turn into a
/// weak solver. "Weak" in this context means that the solver would know if a move is a win, loss or
/// draw, but not how many moves it takes to reach that outcome.
///
/// It can take a lot of memory and compute to proof the outcome of a game, so luckily you still get
/// the counts from undecided if you decide to stop the monte carlo search before a proof is
/// reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountWdlSolved {
    /// The board has been proven to be a win for the given player, given perfect play of both
    /// players.
    Win(Player),
    /// The board has been proven to be a draw, given perfect play of both players.
    Draw,
    /// The outcome could not yet be proven to be either a win, loss or draw.
    Undecided(CountWdl),
}

impl CountWdlSolved {
    /// Count of total playouts
    pub(crate) fn total(&self) -> i32 {
        match self {
            CountWdlSolved::Undecided(count) => count.total(),
            CountWdlSolved::Win(_) | CountWdlSolved::Draw => 1,
        }
    }

    /// Convert solved solutions to their underterministic counter part
    pub(crate) fn into_count(self) -> CountWdl {
        match self {
            CountWdlSolved::Undecided(count) => count,
            CountWdlSolved::Draw => CountWdl {
                draws: 1,
                ..CountWdl::default()
            },
            CountWdlSolved::Win(player) => {
                let mut count = CountWdl::default();
                count.report_win_for(player);
                count
            }
        }
    }

    pub fn undecided(&self) -> Option<&CountWdl> {
        match self {
            CountWdlSolved::Win(_) | CountWdlSolved::Draw => None,
            CountWdlSolved::Undecided(count_wdl) => Some(count_wdl),
        }
    }
}

impl Evaluation for CountWdlSolved {
    fn cmp_for(&self, other: &CountWdlSolved, player: Player) -> Ordering {
        match (self, other) {
            (CountWdlSolved::Win(p1), CountWdlSolved::Win(p2)) => {
                if *p1 == *p2 {
                    Ordering::Equal
                } else if *p1 == player {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (CountWdlSolved::Win(p1), _) => {
                if *p1 == player {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (CountWdlSolved::Draw, CountWdlSolved::Win(p2)) => {
                if *p2 == player {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (CountWdlSolved::Draw, CountWdlSolved::Draw) => Ordering::Equal,
            (CountWdlSolved::Draw, CountWdlSolved::Undecided(count)) => {
                0.5.partial_cmp(&count.reward(player)).unwrap()
            }
            (CountWdlSolved::Undecided(c1), CountWdlSolved::Undecided(c2)) => {
                c1.reward(player).partial_cmp(&c2.reward(player)).unwrap()
            }
            (a, b) => b.cmp_for(a, player).reverse(),
        }
    }

    fn selection_weight(&self, parent_eval: &CountWdlSolved, selecting_player: Player) -> f32 {
        let total_visits_parent = parent_eval.total() as f32;
        match self {
            CountWdlSolved::Undecided(count) => count.ucb(total_visits_parent, selecting_player),
            CountWdlSolved::Draw => 0.5,
            CountWdlSolved::Win(winning_player) => {
                if selecting_player == *winning_player {
                    f32::MAX
                } else {
                    0.0
                }
            }
        }
    }

    fn is_solved(&self) -> bool {
        match self {
            CountWdlSolved::Win(_) | CountWdlSolved::Draw => true,
            CountWdlSolved::Undecided(_) => false,
        }
    }

    fn init_from_game_state<M>(state: &GameState<'_, M>) -> Self {
        match state {
            GameState::Moves(_) => CountWdlSolved::Undecided(CountWdl::default()),
            GameState::Draw => CountWdlSolved::Draw,
            GameState::WinPlayerOne => CountWdlSolved::Win(Player::One),
            GameState::WinPlayerTwo => CountWdlSolved::Win(Player::Two),
        }
    }
}

impl Default for CountWdlSolved {
    fn default() -> Self {
        Self::Undecided(CountWdl::default())
    }
}

/// Delta propagated upwards from child to parent during backpropagation.
pub struct CountWdlSolvedDelta {
    /// Did the child change to a win for either player? Is it a draw? In case of undecided the
    /// count is **not** the count of the child, but the count of the change in the child.
    pub propagated_evaluation: CountWdlSolved,
    /// The count of the child before the change. We can assume the child has been in the
    /// [`CountOrDecided::Undecided`] state before the change. Otherwise it would not have been
    /// selected for expansion.
    pub previous_count: CountWdl,
}

#[cfg(test)]
mod test {
    use std::cmp::Ordering;

    use crate::{CountWdl, CountWdlSolved, Evaluation as _, Player};

    #[test]
    fn compare_evaluations() {
        let win_player_one = CountWdlSolved::Win(Player::One);
        let win_player_two = CountWdlSolved::Win(Player::Two);
        let draw = CountWdlSolved::Draw;
        let one = Player::One;
        let two = Player::Two;

        assert_eq!(
            win_player_one.cmp_for(&win_player_one, one),
            Ordering::Equal
        );
        assert_eq!(
            win_player_one.cmp_for(&win_player_two, one),
            Ordering::Greater
        );
        assert_eq!(win_player_one.cmp_for(&win_player_two, two), Ordering::Less);
        assert_eq!(win_player_one.cmp_for(&draw, one), Ordering::Greater);
        assert_eq!(win_player_one.cmp_for(&draw, two), Ordering::Less);
        assert_eq!(draw.cmp_for(&win_player_one, one), Ordering::Less);
        assert_eq!(draw.cmp_for(&win_player_two, one), Ordering::Greater);
        assert_eq!(draw.cmp_for(&draw, one), Ordering::Equal);
        assert_eq!(
            draw.cmp_for(
                &CountWdlSolved::Undecided(CountWdl {
                    draws: 1,
                    ..CountWdl::default()
                }),
                one
            ),
            Ordering::Equal
        );
        assert_eq!(
            draw.cmp_for(
                &CountWdlSolved::Undecided(CountWdl {
                    wins_player_one: 1,
                    ..CountWdl::default()
                }),
                one
            ),
            Ordering::Less
        );
        assert_eq!(
            CountWdlSolved::Undecided(CountWdl {
                wins_player_one: 1,
                ..CountWdl::default()
            })
            .cmp_for(&win_player_one, one),
            Ordering::Less
        );
        assert_eq!(
            CountWdlSolved::Undecided(CountWdl {
                wins_player_two: 1,
                ..CountWdl::default()
            })
            .cmp_for(&win_player_two, one),
            Ordering::Greater
        );
        assert_eq!(
            CountWdlSolved::Undecided(CountWdl {
                wins_player_one: 1,
                ..CountWdl::default()
            })
            .cmp_for(
                &CountWdlSolved::Undecided(CountWdl {
                    wins_player_two: 1,
                    ..CountWdl::default()
                }),
                one
            ),
            Ordering::Greater
        );
    }
}
