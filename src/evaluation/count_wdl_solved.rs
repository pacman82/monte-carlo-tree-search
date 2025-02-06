use std::cmp::Ordering;

use crate::{GameState, Player};

use super::{Evaluation, CountWdl};

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
}

impl Evaluation for CountWdlSolved {
    type Delta = CountOrDecidedDelta;

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

    /// Called during backpropagation. Updates the evaluation of a node based on a propagated delta
    /// emitted by the update of a child node. In addition to that, we can also take the evaluations
    /// of the siblings of the changed child into account. The method changes the evaluation of the
    /// current node during propagation to its new value. In additon to that it emmits a delta which
    /// in turn is passed to the update of its parent node.
    fn update(
        &mut self,
        sibling_evaluations: impl Iterator<Item = Option<CountWdlSolved>>,
        propagated_delta: CountOrDecidedDelta,
        choosing_player: Player,
    ) -> CountOrDecidedDelta {
        let previous_count = self.into_count();
        let CountOrDecidedDelta {
            propagated_evaluation,
            previous_count: previous_child_count,
        } = propagated_delta;
        if propagated_evaluation == CountWdlSolved::Win(choosing_player) {
            // If it is the choosing players turn, she will choose a win
            *self = propagated_evaluation;
            return CountOrDecidedDelta {
                propagated_evaluation,
                previous_count,
            };
        }
        // If the choosing player is not guaranteed to win let's check if there is a draw or a loss
        let loss = CountWdlSolved::Win(choosing_player.opponent());
        if propagated_evaluation.is_solved() {
            let mut acc = Some(propagated_evaluation);
            for maybe_eval in sibling_evaluations {
                let Some(child_eval) = maybe_eval else {
                    // Still has unexplored children, so we can not be sure the current node is a
                    // draw or a loss.
                    acc = None;
                    break;
                };
                if child_eval == CountWdlSolved::Draw {
                    // Found a draw, so we can be sure its not a loss
                    acc = Some(CountWdlSolved::Draw);
                } else if child_eval != loss {
                    // Found a child neither draw or loss, so we can not rule out a victory yet
                    acc = None;
                    break;
                }
            }
            if let Some(evaluation) = acc {
                *self = evaluation;
                return CountOrDecidedDelta {
                    propagated_evaluation: evaluation,
                    previous_count,
                };
            }
        }
        // No deterministic outcome, let's propagete the counts
        let propageted_count = match propagated_evaluation {
            CountWdlSolved::Win(Player::One) => {
                let mut count = CountWdl {
                    wins_player_one: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountWdlSolved::Win(Player::Two) => {
                let mut count = CountWdl {
                    wins_player_two: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountWdlSolved::Draw => {
                let mut count = CountWdl {
                    draws: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountWdlSolved::Undecided(count) => count,
        };

        let (new_eval, delta) = match self {
            CountWdlSolved::Undecided(mut count) => {
                count += propageted_count;
                (
                    CountWdlSolved::Undecided(count),
                    CountOrDecidedDelta {
                        propagated_evaluation: CountWdlSolved::Undecided(propageted_count),
                        previous_count,
                    },
                )
            }
            _ => (
                *self,
                CountOrDecidedDelta {
                    propagated_evaluation: CountWdlSolved::Undecided(propageted_count),
                    previous_count,
                },
            ),
        };
        *self = new_eval;
        delta
    }

    fn is_solved(&self) -> bool {
        match self {
            CountWdlSolved::Win(_) | CountWdlSolved::Draw => true,
            CountWdlSolved::Undecided(_) => false,
        }
    }

    fn initial_delta(&self) -> Self::Delta {
        CountOrDecidedDelta {
            propagated_evaluation: *self,
            previous_count: CountWdl::default(),
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
pub struct CountOrDecidedDelta {
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

    use crate::{Evaluation as _, Player, CountWdl, CountWdlSolved};

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
