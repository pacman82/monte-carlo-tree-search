use std::{
    cmp::Ordering,
    ops::{AddAssign, SubAssign},
};

use crate::{GameState, Player};

/// Controls what information is stored for each board remembered in the nodes of the tree, how
/// to change it during backpropagation and what criteria to use to select the next node to expand.
pub trait Evaluation: Copy {
    /// Used during backpropagation to pass information from a child node to its parent.
    type Delta;

    /// Define an ordering between two evaluations, so that the greates value is the most favorable
    /// move for the given player. This method is currently used by [`crate::Tree`] in order to
    /// update the best move found so far after each playout.
    fn cmp_for(&self, other: &Self, player: Player) -> Ordering;

    /// A weight used to decide how much we want to explore this node, compared to its siblings.
    /// Higher weightns make a node more likely to be selected.
    fn selection_weight(&self, parent_eval: &Self, selecting_player: Player) -> f32;

    /// Called during backpropagation. Updates the evaluation of a node based on a propagated delta
    /// emitted by the update of a child node. In addition to that, we can also take the evaluations
    /// of the siblings of the changed child into account. The method changes the evaluation of the
    /// current node during propagation to its new value. In additon to that it emmits a delta which
    /// in turn is passed to the update of its parent node.
    fn update(
        &mut self,
        sibling_evaluations: impl Iterator<Item = Option<Self>>,
        propagated_delta: Self::Delta,
        choosing_player: Player,
    ) -> Self::Delta;

    /// Solved states will be ignored during selection phase. If there are no unsolved nodes left
    /// in the tree the search will stop.
    fn is_solved(&self) -> bool;

    /// Initial delto for backpropagation based on the bias found for the new node.
    fn initial_delta(&self) -> Self::Delta;

    /// Creating an initial evaluation for the root node, or before the first simulation. Can be
    /// used to handle terminal states.
    fn init_from_game_state<M>(state: &GameState<'_, M>) -> Self;
}

impl Evaluation for CountOrDecided {
    type Delta = CountOrDecidedDelta;

    fn cmp_for(&self, other: &CountOrDecided, player: Player) -> Ordering {
        match (self, other) {
            (CountOrDecided::Win(p1), CountOrDecided::Win(p2)) => {
                if *p1 == *p2 {
                    Ordering::Equal
                } else if *p1 == player {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (CountOrDecided::Win(p1), _) => {
                if *p1 == player {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (CountOrDecided::Draw, CountOrDecided::Win(p2)) => {
                if *p2 == player {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (CountOrDecided::Draw, CountOrDecided::Draw) => Ordering::Equal,
            (CountOrDecided::Draw, CountOrDecided::Undecided(count)) => {
                0.5.partial_cmp(&count.reward(player)).unwrap()
            }
            (CountOrDecided::Undecided(c1), CountOrDecided::Undecided(c2)) => {
                c1.reward(player).partial_cmp(&c2.reward(player)).unwrap()
            }
            (a, b) => b.cmp_for(a, player).reverse(),
        }
    }

    fn selection_weight(&self, parent_eval: &CountOrDecided, selecting_player: Player) -> f32 {
        let total_visits_parent = parent_eval.total() as f32;
        match self {
            CountOrDecided::Undecided(count) => count.ucb(total_visits_parent, selecting_player),
            CountOrDecided::Draw => 0.5,
            CountOrDecided::Win(winning_player) => {
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
        sibling_evaluations: impl Iterator<Item = Option<CountOrDecided>>,
        propagated_delta: CountOrDecidedDelta,
        choosing_player: Player,
    ) -> CountOrDecidedDelta {
        let previous_count = self.into_count();
        let CountOrDecidedDelta {
            propagated_evaluation,
            previous_count: previous_child_count,
        } = propagated_delta;
        if propagated_evaluation == CountOrDecided::Win(choosing_player) {
            // If it is the choosing players turn, she will choose a win
            *self = propagated_evaluation;
            return CountOrDecidedDelta {
                propagated_evaluation,
                previous_count,
            };
        }
        // If the choosing player is not guaranteed to win let's check if there is a draw or a loss
        let loss = CountOrDecided::Win(choosing_player.opponent());
        if propagated_evaluation.is_solved() {
            let mut acc = Some(propagated_evaluation);
            for maybe_eval in sibling_evaluations {
                let Some(child_eval) = maybe_eval else {
                    // Still has unexplored children, so we can not be sure the current node is a
                    // draw or a loss.
                    acc = None;
                    break;
                };
                if child_eval == CountOrDecided::Draw {
                    // Found a draw, so we can be sure its not a loss
                    acc = Some(CountOrDecided::Draw);
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
            CountOrDecided::Win(Player::One) => {
                let mut count = Count {
                    wins_player_one: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountOrDecided::Win(Player::Two) => {
                let mut count = Count {
                    wins_player_two: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountOrDecided::Draw => {
                let mut count = Count {
                    draws: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountOrDecided::Undecided(count) => count,
        };

        let (new_eval, delta) = match self {
            CountOrDecided::Undecided(mut count) => {
                count += propageted_count;
                (
                    CountOrDecided::Undecided(count),
                    CountOrDecidedDelta {
                        propagated_evaluation: CountOrDecided::Undecided(propageted_count),
                        previous_count,
                    },
                )
            }
            _ => (
                *self,
                CountOrDecidedDelta {
                    propagated_evaluation: CountOrDecided::Undecided(propageted_count),
                    previous_count,
                },
            ),
        };
        *self = new_eval;
        delta
    }
    
    fn is_solved(&self) -> bool {
        match self {
            CountOrDecided::Win(_) | CountOrDecided::Draw => true,
            CountOrDecided::Undecided(_) => false,
        }
    }

    fn initial_delta(&self) -> Self::Delta {
        CountOrDecidedDelta {
            propagated_evaluation: *self,
            previous_count: Count::default(),
        }
    }
    
    fn init_from_game_state<M>(state: &GameState<'_, M>) -> Self {
        match state {
            GameState::Moves(_) => CountOrDecided::Undecided(Count::default()),
            GameState::Draw => CountOrDecided::Draw,
            GameState::WinPlayerOne => CountOrDecided::Win(Player::One),
            GameState::WinPlayerTwo => CountOrDecided::Win(Player::Two),
        }
    }
}

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
pub enum CountOrDecided {
    /// The board has been proven to be a win for the given player, given perfect play of both
    /// players.
    Win(Player),
    /// The board has been proven to be a draw, given perfect play of both players.
    Draw,
    /// The outcome could not yet be proven to be either a win, loss or draw.
    Undecided(Count),
}

impl CountOrDecided {
    /// Count of total playouts
    pub(crate) fn total(&self) -> i32 {
        match self {
            CountOrDecided::Undecided(count) => count.total(),
            CountOrDecided::Win(_) | CountOrDecided::Draw => 1,
        }
    }

    /// Convert solved solutions to their underterministic counter part
    pub(crate) fn into_count(self) -> Count {
        match self {
            CountOrDecided::Undecided(count) => count,
            CountOrDecided::Draw => Count {
                draws: 1,
                ..Count::default()
            },
            CountOrDecided::Win(player) => {
                let mut count = Count::default();
                count.report_win_for(player);
                count
            }
        }
    }
}

impl Default for CountOrDecided {
    fn default() -> Self {
        Self::Undecided(Count::default())
    }
}

/// Delta propagated upwards from child to parent during backpropagation.
pub struct CountOrDecidedDelta {
    /// Did the child change to a win for either player? Is it a draw? In case of undecided the
    /// count is **not** the count of the child, but the count of the change in the child.
    pub propagated_evaluation: CountOrDecided,
    /// The count of the child before the change. We can assume the child has been in the
    /// [`CountOrDecided::Undecided`] state before the change. Otherwise it would not have been
    /// selected for expansion.
    pub previous_count: Count,
}

/// Counts accumulated wins, losses and draws for this part of the tree
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count {
    pub wins_player_one: i32,
    pub wins_player_two: i32,
    pub draws: i32,
}

impl Count {
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

impl AddAssign for Count {
    fn add_assign(&mut self, other: Self) {
        self.wins_player_one += other.wins_player_one;
        self.wins_player_two += other.wins_player_two;
        self.draws += other.draws;
    }
}

impl SubAssign for Count {
    fn sub_assign(&mut self, other: Self) {
        self.wins_player_one -= other.wins_player_one;
        self.wins_player_two -= other.wins_player_two;
        self.draws -= other.draws;
    }
}

#[cfg(test)]
mod test {
    use std::cmp::Ordering;

    use crate::{Count, CountOrDecided, Evaluation as _, Player};

    #[test]
    fn compare_evaluations() {
        let win_player_one = CountOrDecided::Win(Player::One);
        let win_player_two = CountOrDecided::Win(Player::Two);
        let draw = CountOrDecided::Draw;
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
                &CountOrDecided::Undecided(Count {
                    draws: 1,
                    ..Count::default()
                }),
                one
            ),
            Ordering::Equal
        );
        assert_eq!(
            draw.cmp_for(
                &CountOrDecided::Undecided(Count {
                    wins_player_one: 1,
                    ..Count::default()
                }),
                one
            ),
            Ordering::Less
        );
        assert_eq!(
            CountOrDecided::Undecided(Count {
                wins_player_one: 1,
                ..Count::default()
            })
            .cmp_for(&win_player_one, one),
            Ordering::Less
        );
        assert_eq!(
            CountOrDecided::Undecided(Count {
                wins_player_two: 1,
                ..Count::default()
            })
            .cmp_for(&win_player_two, one),
            Ordering::Greater
        );
        assert_eq!(
            CountOrDecided::Undecided(Count {
                wins_player_one: 1,
                ..Count::default()
            })
            .cmp_for(
                &CountOrDecided::Undecided(Count {
                    wins_player_two: 1,
                    ..Count::default()
                }),
                one
            ),
            Ordering::Greater
        );
    }
}
