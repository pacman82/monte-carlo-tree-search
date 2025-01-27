use rand::Rng;

use crate::{simulation, Evaluation, TwoPlayerGame};

/// Used to obtain an ininitial bias for the outcome of a game starting from a given board.
pub trait Bias<G: TwoPlayerGame> {
    fn bias(&self, game: G, move_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> Evaluation;
}

/// Obtain an initial bias by playing random moves and reporting the outcome.
pub struct RandomPlayoutBias;

impl<G> Bias<G> for RandomPlayoutBias
where
    G: TwoPlayerGame,
{
    fn bias(&self, game: G, move_buf: &mut Vec<G::Move>, rng: &mut impl Rng) -> Evaluation {
        Evaluation::Undecided(simulation(game, move_buf, rng))
    }
}
