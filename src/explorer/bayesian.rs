use std::cmp::Ordering;

use rand::Rng;

use crate::{Evaluation, GameState, Player, TwoPlayerGame};

use super::Explorer;

pub struct Bayesian<B> {
    bias: B,
}

impl<B> Bayesian<B> {
    pub fn new() -> Self where B: Default {
        Bayesian {
            bias: B::default(),
        }
    }
}

impl<B, G> Explorer<G> for Bayesian<B>
where
    G: TwoPlayerGame,
    B: ProbabilityWdlBias<G>,
{
    type Evaluation = ProbabilityWdl;

    type Delta = ();

    fn bias(&mut self, game: G, rng: &mut impl Rng) -> ProbabilityWdl {
        self.bias.bias(game, rng)
    }

    fn unexplored_bias(&self) -> ProbabilityWdl {
        ProbabilityWdl::from_ratio(1, 1, 1)
    }

    fn reevaluate(&mut self, game: G, evaluation: &mut ProbabilityWdl) -> Self::Delta {
        todo!()
    }

    fn selected_child_pos<'a>(
        &self,
        parent_eval: &ProbabilityWdl,
        child_evals: impl ExactSizeIterator<Item = &'a ProbabilityWdl>,
        selecting_player: Player,
    ) -> Option<usize> {
        todo!()
    }

    fn update(
        &mut self,
        old_evaluation: &mut ProbabilityWdl,
        sibling_evaluations: impl Iterator<Item = Option<ProbabilityWdl>>,
        propagated_delta: Self::Delta,
        choosing_player: Player,
    ) -> Self::Delta {
        
        todo!()
    }

    fn initial_delta(&self, new_evaluation: &ProbabilityWdl) -> Self::Delta {
        todo!()
    }

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
    fn expectation(&self, player: Player) -> f64 {
        let p_diff = match player {
            Player::One => self.win_player_one - self.win_player_two,
            Player::Two => self.win_player_two - self.win_player_one,
        };
        0.5 * (1.0 + p_diff)
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

pub trait ProbabilityWdlBias<G> {
    fn bias(&mut self, game: G, rng: &mut impl Rng) -> ProbabilityWdl;
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
