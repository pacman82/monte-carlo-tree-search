use std::cmp::Ordering;

use rand::Rng;

use crate::{Evaluation, GameState, Player, TwoPlayerGame};

use super::{CountWdlBias, Explorer};

pub struct Bayesian<S> {
    /// Sampling strategy for simulation phase. Used to caluculate bias for newly explored nodes.
    sampling: S,
    prior_win_one: u32,
    prior_win_two: u32,
    prior_draw: u32,
}

impl<S> Bayesian<S> {
    pub fn new() -> Self
    where
        S: Default,
    {
        Self::with_bias(1, 1, 1, S::default())
    }

    pub fn with_bias(prior_win_one: u32, prior_win_two: u32, prior_draw: u32, sampling: S) -> Self {
        Bayesian {
            sampling,
            prior_win_one,
            prior_win_two,
            prior_draw,
        }
    }
}

impl<S, G> Explorer<G> for Bayesian<S>
where
    S: CountWdlBias<G>,
    G: TwoPlayerGame,
{
    type Evaluation = ProbabilityWdl;

    type Delta = ();

    fn bias(&mut self, game: G, rng: &mut impl Rng) -> ProbabilityWdl {
        let sample = self.sampling.bias(game, rng);
        ProbabilityWdl::from_ratio(
            self.prior_win_one + sample.wins_player_one as u32,
            self.prior_win_two + sample.wins_player_two as u32,
            self.prior_draw + sample.draws as u32,
        )
    }

    fn unexplored_bias(&self) -> ProbabilityWdl {
        ProbabilityWdl::from_ratio(self.prior_win_one, self.prior_win_two, self.prior_draw)
    }

    fn reevaluate(&mut self, _game: G, _evaluation: &mut ProbabilityWdl) -> Self::Delta {
        // Should only be called for terminal states (if at all). In this case the probability
        // should already be very much reflect certainity. We leave it untouched.
    }

    fn selected_child_pos<'a>(
        &self,
        parent_eval: &ProbabilityWdl,
        child_evals: impl ExactSizeIterator<Item = &'a ProbabilityWdl>,
        selecting_player: Player,
    ) -> Option<usize> {
        let total_visits_parent = 100f64; // TODO: Use actual number of visits
        child_evals
            .map(|eval| eval.uct2(selecting_player, total_visits_parent))
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(index, _)| index)
    }

    fn update(
        &mut self,
        old_evaluation: &mut ProbabilityWdl,
        sibling_evaluations: impl Iterator<Item = Option<ProbabilityWdl>>,
        propagated_delta: Self::Delta,
        choosing_player: Player,
    ) -> Self::Delta {
    }

    fn initial_delta(&self, new_evaluation: &ProbabilityWdl) -> Self::Delta {}

    fn is_solved(&self, _evaluation: &ProbabilityWdl) -> bool {
        false
    }
}

/// Probabilities of winning for either player. Probability for draw is implicitly contained.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProbabilityWdl {
    // Probability for player one to win. Should be in [0, 1].
    win_player_one: f64,
    // Probability for player two to win. Should be in [0, 1].
    win_player_two: f64,
}

impl ProbabilityWdl {
    pub fn from_ratio(win_player_one: u32, win_player_two: u32, draw: u32) -> Self {
        let total = win_player_one + win_player_two + draw;
        ProbabilityWdl {
            win_player_one: win_player_one as f64 / total as f64,
            win_player_two: win_player_two as f64 / total as f64,
        }
    }

    /// We assume we value a win as 1 a draw as 0.5, and a loss as 0.
    pub fn expectation(&self, player: Player) -> f64 {
        let p_diff = match player {
            Player::One => self.win_player_one - self.win_player_two,
            Player::Two => self.win_player_two - self.win_player_one,
        };
        0.5 * (1.0 + p_diff)
    }

    pub fn variance(&self, player: Player) -> f64 {
        let (reward_player_one, reward_player_two) = match player {
            Player::One => (1.0, 0.0),
            Player::Two => (0.0, 1.0),
        };
        let mean = self.expectation(player);
        self.win_player_one * (mean - reward_player_one).powi(2)
            + self.win_player_two * (mean - reward_player_two).powi(2)
            - self.draw() * (mean - 0.5).powi(2)
    }

    pub fn deviation(&self, player: Player) -> f64 {
        self.variance(player).sqrt()
    }

    fn draw(&self) -> f64 {
        1.0 - self.win_player_one - self.win_player_two
    }

    /// B = expecation + sqrt(2 * ln(N) * deviation)
    fn uct2(&self, player: Player, total_visits_parent: f64) -> f64 {
        self.expectation(player) + (2f64 * total_visits_parent.ln() * self.deviation(player)).sqrt()
    }
}

impl Evaluation for ProbabilityWdl {
    /// For now let's judge that the best move is the one with the highest expectation.
    fn cmp_for(&self, other: &ProbabilityWdl, player: Player) -> Ordering {
        self.expectation(player)
            .partial_cmp(&other.expectation(player))
            .unwrap()
    }

    fn eval_for_terminal_state<M>(state: &crate::GameState<'_, M>) -> Self {
        match state {
            GameState::Moves(_) => {
                panic!("eval_for_terminal_state must only be called for terminal states")
            }
            GameState::WinPlayerOne => ProbabilityWdl {
                win_player_one: 1.0,
                win_player_two: 0.0,
            },
            GameState::WinPlayerTwo => ProbabilityWdl {
                win_player_one: 0.0,
                win_player_two: 1.0,
            },
            GameState::Draw => ProbabilityWdl {
                win_player_one: 0.0,
                win_player_two: 0.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::Player;

    use super::ProbabilityWdl;

    #[test]
    fn expectation() {
        assert_eq!(
            ProbabilityWdl {
                win_player_one: 0.5,
                win_player_two: 0.5
            }
            .expectation(Player::One),
            0.5
        );

        assert_eq!(
            ProbabilityWdl {
                win_player_one: 1.0,
                win_player_two: 0.0
            }
            .expectation(Player::One),
            1.0
        );

        assert_eq!(
            ProbabilityWdl {
                win_player_one: 1.0,
                win_player_two: 0.0
            }
            .expectation(Player::Two),
            0.0
        );

        assert_eq!(
            ProbabilityWdl {
                win_player_one: 0.0,
                win_player_two: 0.0
            }
            .expectation(Player::Two),
            0.5
        );
    }
}
