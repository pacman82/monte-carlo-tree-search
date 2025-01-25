use std::{io::Write, fmt::{self, Display}};

use connect_four_solver::{Column, Solver};
use monte_carlo_tree_search::{Evaluation, GameState, Player, Tree, TwoPlayerGame};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

#[test]
fn play_move_connect_four() {
    let mut rng = StdRng::seed_from_u64(42);
    let game = ConnectFour::new();
    let num_playouts = 100;

    let tree = Tree::with_playouts(game, num_playouts, &mut rng);

    for (move_, eval) in tree.estimated_outcome_by_move() {
        eprintln!(
            "Eval child {:?}: {:?} ",
            move_,
            eval,
        );
    }
}

#[test]
fn start_from_terminal_position() {
    // First player has won
    let game = ConnectFour::from_move_list("1212121");
    let tree = Tree::new(game);

    assert_eq!(Evaluation::Win(Player::One), tree.evaluation());
}

/// Position occured once letting the tree play against itself, for some reason the solver did not
/// find the obvious winning move (`1`).
#[test]
fn position_42442445555772222514171766() {
    let game = ConnectFour::from_move_list("424424455557722225141717");
    eprintln!("{game}");
    // | |X| |O| | | |
    // | |O| |X|O| | |
    // | |X| |O|X| |O|
    // |X|O| |O|O| |O|
    // |X|X| |X|X| |X|
    // |X|O| |X|O| |O|
    // ---------------
    //  1 2 3 4 5 6 7

    let mut rng = StdRng::seed_from_u64(42);
    let num_playouts = 1_000;
    let tree = Tree::with_playouts(game, num_playouts, &mut rng);
    print_move_statistics(&tree);
    assert_eq!(Column::from_index(0), tree.best_move().unwrap());
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
            tree.best_move().unwrap()
        } else {
            solver.best_moves(&game, &mut moves);
            *moves.choose(&mut rng).unwrap()
        };
        eprintln!("column: {next_move}");
        game.play(next_move);
        eprintln!("{game}");
    }
}

#[test]
#[ignore = "Computes a long time. More a design exploration, than an actual test"]
fn play_against_yourself() {
    let mut rng = StdRng::seed_from_u64(42);

    let mut game = connect_four_solver::ConnectFour::new();

    let num_playouts_player_one = 10_000;
    let num_playouts_player_two = 1_000;
    let mut history = Vec::new();

    while !game.is_over() {
        let num_playouts = if game.stones() % 2 == 0 {
            num_playouts_player_one
        } else {
            num_playouts_player_two
        };
        let tree = Tree::with_playouts(ConnectFour(game), num_playouts, &mut rng);
        eprintln!("nodes: {} links: {}", tree.num_nodes(), tree.num_links());
        let next_move = tree.best_move().unwrap();
        eprintln!("column: {next_move}");
        write!(&mut history, "{next_move}").unwrap();
        game.play(next_move);
        eprintln!("{game}");
    }
    eprint!("History: {}", String::from_utf8(history).unwrap());
}

fn print_move_statistics(tree: &Tree<ConnectFour>) {
    let evals = tree.estimated_outcome_by_move().collect::<Vec<_>>();
    for (mv, eval) in evals {
        eprintln!(
            "Move: {:?} Eval: {:?}",
            mv,
            eval,
        );
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

    fn current_player(&self) -> Player {
        if self.0.stones() % 2 == 0 {
            Player::One
        } else {
            Player::Two
        }
    }
}
