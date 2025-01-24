use std::fmt::{self, Display};

use connect_four_solver::{Column, Solver};
use monte_carlo_tree_search::{Evaluation, GameState, Tree, TwoPlayerGame};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

#[test]
fn play_move_connect_four() {
    let mut rng = StdRng::seed_from_u64(42);
    let game = ConnectFour::new();
    let num_playouts = 100;

    let tree = Tree::with_playouts(game, num_playouts, &mut rng);

    for (move_, score) in tree.estimated_outcome_by_move() {
        eprintln!(
            "Score child {:?}: {:?} Reward: {:?}",
            move_,
            score,
            score.reward(0)
        );
    }
}

#[test]
fn start_from_terminal_position() {
    // First player has won
    let game = ConnectFour::from_move_list("1212121");
    let tree = Tree::new(game);

    assert_eq!(
        Evaluation::WinPlayerOne,
        tree.estimate_outcome()
    );
}

#[test]
#[ignore = "Computes a long time. More a design exploration, than an actual test"]
fn play_against_perfect_solver_as_player_one() {
    let mut rng = StdRng::seed_from_u64(42);

    let mut game = connect_four_solver::ConnectFour::new();
    let mut solver = Solver::new();
    let mut moves = Vec::new();

    while !game.is_over() {
        let next_move = if game.stones() % 2 == 0 {
            let num_playouts = 1_000;
            let tree = Tree::with_playouts(ConnectFour(game), num_playouts, &mut rng);
            eprintln!("nodes: {} links: {}", tree.num_nodes(), tree.num_links());
            tree.estimated_outcome_by_move()
                .max_by(|(_, score_a), (_, score_b)| {
                    let a = score_a.reward(0);
                    let b = score_b.reward(0);
                    a.partial_cmp(&b).unwrap()
                })
                .unwrap()
                .0
        } else {
            solver.best_moves(&game, &mut moves);
            *moves.choose(&mut rng).unwrap()
        };
        eprintln!("column: {next_move}");
        game.play(next_move);
        eprintln!("{game}");
    }
}

/// Newtype for [`connect_four_solver::ConnectFour`], so we can implement `TwoPlayerGame` for it.
#[derive(Clone, Copy)]
struct ConnectFour(connect_four_solver::ConnectFour);

impl ConnectFour {
    pub fn new() -> Self {
        ConnectFour(connect_four_solver::ConnectFour::new())
    }

    pub fn from_move_list(move_list: &str) -> Self {
        ConnectFour(connect_four_solver::ConnectFour::from_move_list(move_list))
    }
}

impl Display for ConnectFour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TwoPlayerGame for ConnectFour {
    type Move = Column;

    fn state<'a>(&self, moves_buf: &'a mut Vec<Column>) -> GameState<'a, Column> {
        if self.0.is_victory() {
            // Convention for `GameState` is different than the one for `is_victory`. `is_victory`
            // is from the perspective of the player which played the last stone.
            if self.0.stones() % 2 == 0 {
                return GameState::WinPlayerTwo;
            } else {
                return GameState::WinPlayerOne;
            }
        }
        if self.0.stones() == 42 {
            return GameState::Draw;
        }
        moves_buf.clear();
        moves_buf.extend(self.0.legal_moves());
        GameState::Moves(moves_buf.as_slice())
    }

    fn play(&mut self, column: &Column) {
        self.0.play(*column);
    }

    fn current_player(&self) -> u8 {
        self.0.stones() % 2
    }
}
