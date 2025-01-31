use std::{
    fmt::{self, Display},
    io::Write,
};

use connect_four_solver::{Column, Solver};
use monte_carlo_tree_search::{
    Bias, Count, Ucb, GameState, Player, RandomPlayoutBias, Tree, TwoPlayerGame,
};
use rand::{rngs::StdRng, seq::IndexedRandom as _, Rng, SeedableRng};

#[test]
fn play_move_connect_four() {
    let mut rng = StdRng::seed_from_u64(42);
    let game = ConnectFour::new();
    let num_playouts = 100;

    let tree = Tree::with_playouts(game, RandomPlayoutBias, num_playouts, &mut rng);

    for (move_, eval) in tree.eval_by_move() {
        eprintln!("Eval child {:?}: {:?} ", move_, eval,);
    }
}

#[test]
fn start_from_terminal_position() {
    // First player has won
    let game = ConnectFour::from_move_list("1212121");
    let tree = Tree::new(game, RandomPlayoutBias);

    assert_eq!(Ucb::Win(Player::One), tree.evaluation());
}

/// Position occured once letting the tree play against itself, for some reason the solver did not
/// find the obvious winning move (`1`).
#[test]
fn position_424424455557722225141717() {
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
    let tree = Tree::with_playouts(game, RandomPlayoutBias, num_playouts, &mut rng);
    print_move_statistics(&tree);
    assert_eq!(Column::from_index(0), tree.best_move().unwrap());
}

/// `O` needs to play `1` in order to prevent `X` from winning via `1`. However, no matter what `O`
/// plays, `X` will win. This test verifies it still plays `1`, even though all outcomes are the
/// same considering we only solve the game weekly.
#[test]
fn position_42442445555772222514171() {
    let game = ConnectFour::from_move_list("42442445555772222514171");
    eprintln!("{game}");
    // | |X| |O| | | |
    // | |O| |X|O| | |
    // | |X| |O|X| | |
    // |X|O| |O|O| |O|
    // |X|X| |X|X| |X|
    // |X|O| |X|O| |O|
    // ---------------
    //  1 2 3 4 5 6 7

    let mut rng = StdRng::seed_from_u64(42);
    // 1000 playouts are not enough to prove every move is a loss for `O`.
    let num_playouts = 1000;
    let tree = Tree::with_playouts(game, RandomPlayoutBias, num_playouts, &mut rng);
    print_move_statistics(&tree);
    assert!(tree
        .eval_by_move()
        .all(|(_move, eval)| eval == Ucb::Win(Player::One)));
    assert_eq!(Column::from_index(0), tree.best_move().unwrap());
}

#[test]
#[ignore = "Computes a long time. More a design exploration, than an actual test"]
fn beat_perfect_solver_as_player_one() {
    let mut rng = StdRng::seed_from_u64(42);

    let mut game = ConnectFour::new();
    let mut solver = Solver::new();
    let mut moves = Vec::new();

    while !game.0.is_over() {
        let next_move = match game.current_player() {
            Player::One => {
                let num_playouts = 20_000;
                let tree = Tree::with_playouts(game, ConnectFourBias, num_playouts, &mut rng);
                eprintln!("nodes: {} links: {}", tree.num_nodes(), tree.num_links());
                print_move_statistics(&tree);
                tree.best_move().unwrap()
            }
            Player::Two => {
                solver.best_moves(&game.0, &mut moves);
                *moves.choose(&mut rng).unwrap()
            }
        };
        eprintln!("column: {next_move}");
        game.play(&next_move);
        eprintln!("{game}");
    }

    assert!(game.0.is_victory());
    assert_eq!(game.current_player(), Player::Two);
}

#[test]
#[ignore = "Computes a long time. More a design exploration, than an actual test"]
fn play_against_yourself() {
    let mut rng = StdRng::seed_from_u64(5);
    let mut game = connect_four_solver::ConnectFour::new();

    let mut history = Vec::new();

    while !game.is_over() {
        let next_move = if game.stones() % 2 == 0 {
            // Player One
            eprintln!("Player One");
            let bias = ConnectFourBias;
            let num_playouts = 100_000;
            use_tree_to_generate_move(game, num_playouts, bias, &mut rng)
        } else {
            // Player Two
            eprintln!("Player Two");
            let bias = RandomPlayoutBias;
            let num_playouts = 100_000;
            use_tree_to_generate_move(game, num_playouts, bias, &mut rng)
        };
        eprintln!("column: {next_move}");
        write!(&mut history, "{next_move}").unwrap();
        game.play(next_move);
        eprintln!("{game}");
    }
    eprint!("History: {}", String::from_utf8(history).unwrap());
}

#[test]
#[should_panic]
#[ignore = "Not powerful enough to solve the game, yet. Takes a long time."]
fn solve_connect_four() {
    let mut rng = StdRng::seed_from_u64(42);
    let game = ConnectFour::new();
    let num_playouts = 100_000_000;

    let tree = Tree::with_playouts(game, ConnectFourBias, num_playouts, &mut rng);

    assert_eq!(Ucb::Win(Player::One), tree.evaluation());
}

fn print_move_statistics<B>(tree: &Tree<ConnectFour, B>) {
    let evals = tree.eval_by_move().collect::<Vec<_>>();
    for (mv, eval) in evals {
        eprintln!("Move: {:?} Eval: {:?}", mv, eval,);
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

/// Uses a random playout to generate a bias. However the random players will take a winning move if
/// any is possible. A player will also prefer move wich prevent his opponent from winning
/// immediately on his/her next move, if possible.
struct ConnectFourBias;

impl Bias<ConnectFour> for ConnectFourBias {
    type Evaluation = Ucb;

    fn bias(
        &self,
        mut game: ConnectFour,
        move_buf: &mut Vec<Column>,
        rng: &mut impl Rng,
    ) -> Ucb {
        // Check for terminal position. Actually this should never be used, as bias should only be
        // invoked on non-terminal positions.
        debug_assert!(!game.0.is_victory());
        // If the current player can win in the next move, we can deterministically say that this
        // board evaluates to a win for this player.
        if game.0.can_win_in_next_move() {
            return Ucb::Win(game.current_player());
        }
        if game.0.non_loosing_moves().next().is_none() {
            return Ucb::Win(game.current_player().opponent());
        }
        loop {
            match game.state(move_buf) {
                GameState::Moves(_moves) => (),
                GameState::WinPlayerOne | GameState::WinPlayerTwo => {
                    unreachable!("Should have detected winning move beforehand")
                }
                GameState::Draw => {
                    return Ucb::Undecided(Count {
                        draws: 1,
                        ..Count::default()
                    })
                }
            }
            if game.0.can_win_in_next_move() {
                let mut count = Count::default();
                count.report_win_for(game.current_player());
                return Ucb::Undecided(count);
            }
            move_buf.clear();
            move_buf.extend(game.0.non_loosing_moves());
            if let Some(next_move) = move_buf.choose(rng) {
                game.play(next_move);
            } else {
                // No move available which would not allow our opponent to win, so we loose.
                let mut count = Count::default();
                count.report_win_for(game.current_player().opponent());
                return Ucb::Undecided(count);
            }
        }
    }
}

fn use_tree_to_generate_move<B>(
    game: connect_four_solver::ConnectFour,
    num_playouts: u32,
    bias: B,
    rng: &mut impl Rng,
) -> Column
where
    B: Bias<ConnectFour>,
{
    let tree = Tree::with_playouts(ConnectFour(game), bias, num_playouts, rng);
    eprintln!("nodes: {} links: {}", tree.num_nodes(), tree.num_links());
    print_move_statistics(&tree);
    tree.best_move().unwrap()
}
