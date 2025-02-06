use std::ops::{Deref, DerefMut};

use monte_carlo_tree_search::{
    CountWdlSolved, GameState, Player, Policy, RandomPlayout, Tree, TwoPlayerGame, Ucb, UcbSolver,
};
use rand::{rngs::StdRng, SeedableRng as _};
use tic_tac_toe_board::{CellIndex, TicTacToeState};

#[test]
fn play_tic_tac_toe_using_ucb_solver() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut game = TicTacToe::new();

    let num_playouts = 1_000;
    while !game.0.state().is_terminal() {
        let tree = Tree::with_playouts(
            game,
            UcbSolver::<RandomPlayout<_>>::new(),
            num_playouts,
            &mut rng,
        );
        let best_move = tree.best_move().unwrap();
        game.play_move(&best_move);
        // use std::io::stderr;
        // game.print_to(stderr()).unwrap();
        // eprintln!();
    }

    let mut moves_buf = Vec::new();
    assert_eq!(GameState::Draw, game.state(&mut moves_buf));
}

#[test]
fn play_tic_tac_toe_using_ucb() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut game = TicTacToe::new();

    let num_playouts = 1_000;
    while !game.0.state().is_terminal() {
        let tree = Tree::with_playouts(game, Ucb::new(), num_playouts, &mut rng);
        let best_move = tree.best_move().unwrap();
        game.play_move(&best_move);
        // use std::io::stderr;
        // game.print_to(stderr()).unwrap();
        // eprintln!();
    }
}

#[test]
fn backpropagation_of_draw() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(2));

    let num_playouts = 1_000;

    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );
    let best_move = tree.best_move().unwrap();
    game.play_move(&best_move);

    print_move_statistics(&tree);
    assert_eq!(CountWdlSolved::Draw, tree.evaluation());
}

#[test]
fn solve_tic_tac_toe() {
    let mut rng = StdRng::seed_from_u64(42);
    let game = TicTacToe::new();

    // 362_880 is faculty 9, so it should be an upper bound for the number of playouts needed to
    // solve the game. In actuality, we are likely to solve it with fewer playouts, as will be
    // indicated by the number of nodes in the tree.
    let num_playouts = 362_880;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );
    eprintln!("nodes: {} links: {}", tree.num_nodes(), tree.num_links());
    print_move_statistics(&tree);

    assert_eq!(CountWdlSolved::Draw, tree.evaluation());
}

#[test]
fn prevent_immediate_win_of_player_two() {
    let mut rng = StdRng::seed_from_u64(42);
    // -------
    // | | |X|
    // |-----|
    // | |X| |
    // |-----|
    // |O| |O|
    // -------
    // X must play `7`, to prevent O from winning in the next turn.
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(6));
    game.play_move(&CellIndex::new(2));
    game.play_move(&CellIndex::new(8));

    // use std::io::stderr;
    // game.print_to(stderr()).unwrap();

    let num_playouts = 34;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );
    print_move_statistics(&tree);
    assert_eq!(CellIndex::new(7), tree.best_move().unwrap());
}

#[test]
fn prevent_immediate_win_of_player_one() {
    let mut rng = StdRng::seed_from_u64(42);
    // -------
    // | | |X|
    // |-----|
    // | |X| |
    // |-----|
    // |O|X|O|
    // -------
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(6));
    game.play_move(&CellIndex::new(2));
    game.play_move(&CellIndex::new(8));
    game.play_move(&CellIndex::new(7));

    // use std::io::stderr;
    // game.print_to(stderr()).unwrap();

    let num_playouts = 25;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );
    print_move_statistics(&tree);
    assert_eq!(CellIndex::new(1), tree.best_move().unwrap());
}

fn print_move_statistics<B>(tree: &Tree<TicTacToe, B>)
where
    B: Policy<TicTacToe, Evaluation = CountWdlSolved>,
{
    let evals = tree.eval_by_move().collect::<Vec<_>>();
    for (mv, eval) in evals {
        eprintln!("Move: {:?} Count: {:?}", mv, eval,);
    }
}

#[test]
fn report_win_if_initialized_with_terminal_position() {
    let mut rng = StdRng::seed_from_u64(42);
    // -------
    // |X|O|O|
    // |-----|
    // |O|X| |
    // |-----|
    // |X| |X|
    // -------
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(1));
    game.play_move(&CellIndex::new(6));
    game.play_move(&CellIndex::new(2));
    game.play_move(&CellIndex::new(0));
    game.play_move(&CellIndex::new(3));
    game.play_move(&CellIndex::new(8));
    // game.print_to(stderr()).unwrap();

    let num_playouts = 1;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );

    assert_eq!(CountWdlSolved::Win(Player::One), tree.evaluation())
}

#[test]
fn solve_draw_in_one_move() {
    let mut rng = StdRng::seed_from_u64(0);
    // -------
    // |O|X|O|
    // |-----|
    // |X|X|O|
    // |-----|
    // |X|O|8|
    // -------
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(0));
    game.play_move(&CellIndex::new(1));
    game.play_move(&CellIndex::new(7));
    game.play_move(&CellIndex::new(6));
    game.play_move(&CellIndex::new(2));
    game.play_move(&CellIndex::new(3));
    game.play_move(&CellIndex::new(5));

    // game.print_to(stderr()).unwrap();

    // RNG works out in a way, that if we seed 42 this would work with one playout
    let num_playouts = 1;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );

    print_move_statistics(&tree);
    assert_eq!(CountWdlSolved::Draw, tree.evaluation());
    assert_eq!(CellIndex::new(8), tree.best_move().unwrap())
}

#[test]
fn solve_draw_in_two_moves() {
    let mut rng = StdRng::seed_from_u64(0);
    // -------
    // |O|X|O|
    // |-----|
    // |X|X|5|
    // |-----|
    // |X|O|8|
    // -------
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(0));
    game.play_move(&CellIndex::new(1));
    game.play_move(&CellIndex::new(7));
    game.play_move(&CellIndex::new(6));
    game.play_move(&CellIndex::new(2));
    game.play_move(&CellIndex::new(3));

    // game.print_to(stderr()).unwrap();

    // RNG works out in a way, that if we seed 42 this would work with one playout
    let num_playouts = 4;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );

    print_move_statistics(&tree);
    assert_eq!(CountWdlSolved::Draw, tree.evaluation());
    assert_eq!(CellIndex::new(5), tree.best_move().unwrap())
}

#[test]
fn solve_draw_in_three_moves() {
    let mut rng = StdRng::seed_from_u64(0);
    // -------
    // |O|X|O|
    // |-----|
    // |3|X|5|
    // |-----|
    // |X|O|8|
    // -------
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(0));
    game.play_move(&CellIndex::new(1));
    game.play_move(&CellIndex::new(7));
    game.play_move(&CellIndex::new(6));
    game.play_move(&CellIndex::new(2));

    // game.print_to(stderr()).unwrap();

    // RNG works out in a way, that if we seed 42 this would work with one playout
    let num_playouts = 15;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );

    print_move_statistics(&tree);
    assert_eq!(CountWdlSolved::Draw, tree.evaluation());
}

#[test]
fn solve_win_in_one_move() {
    let mut rng = StdRng::seed_from_u64(0);
    // -------
    // |X|O|O|
    // |-----|
    // |3|X|5|
    // |-----|
    // |X|7|O|
    // -------
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(1));
    game.play_move(&CellIndex::new(6));
    game.play_move(&CellIndex::new(2));
    game.play_move(&CellIndex::new(0));
    game.play_move(&CellIndex::new(8));
    // game.print_to(stderr()).unwrap();

    // RNG works out in a way, that if we seed 42 this would work with one playout
    let num_playouts = 3;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );

    assert_eq!(CountWdlSolved::Win(Player::One), tree.evaluation());
    assert_eq!(CellIndex::new(3), tree.best_move().unwrap())
}

#[test]
fn solve_defeat_in_two_moves() {
    let mut rng = StdRng::seed_from_u64(0);
    // -------
    // |X|O|O|
    // |-----|
    // |3|X|5|
    // |-----|
    // |X|7|8|
    // -------
    // X has two possibilities to win, 3 and 8. So no matter what O plays, X will win.
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(1));
    game.play_move(&CellIndex::new(6));
    game.play_move(&CellIndex::new(2));
    game.play_move(&CellIndex::new(0));
    // game.print_to(stderr()).unwrap();

    let num_playouts = 15;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );

    assert_eq!(CountWdlSolved::Win(Player::One), tree.evaluation());
    print_move_statistics(&tree);
}

#[test]
fn solve_win_in_five_moves() {
    let mut rng = StdRng::seed_from_u64(42);
    // -------
    // |0|O|2|
    // |-----|
    // |3|X|5|
    // |-----|
    // |6|7|8|
    // -------
    // X has several winning moves here
    let mut game = TicTacToe::new();
    game.play_move(&CellIndex::new(4));
    game.play_move(&CellIndex::new(1));
    // game.print_to(stderr()).unwrap();

    let num_playouts = 312;
    let tree = Tree::with_playouts(
        game,
        UcbSolver::<RandomPlayout<_>>::new(),
        num_playouts,
        &mut rng,
    );

    print_move_statistics(&tree);
    assert_eq!(CountWdlSolved::Win(Player::One), tree.evaluation());
}

/// With few or zero playouts, we can be in a situation, there not all nodes of the root are
/// explored. We want to handle unexplored direct children of the root node, withouth panic.
#[test]
fn unexplored_root_childs() {
    let game = TicTacToe::new();

    let tree = Tree::new(game, UcbSolver::<RandomPlayout<_>>::new());

    assert!(tree.best_move().is_some());
    // Just iterate to see that we do not panic in case child is unexplored
    assert_eq!(9, tree.eval_by_move().count());
}

/// Strict alias, so we can implement trait for type
#[derive(Clone, Copy)]
struct TicTacToe(tic_tac_toe_board::TicTacToe);

impl TicTacToe {
    pub fn new() -> Self {
        TicTacToe(tic_tac_toe_board::TicTacToe::new())
    }
}

impl Deref for TicTacToe {
    type Target = tic_tac_toe_board::TicTacToe;

    fn deref(&self) -> &tic_tac_toe_board::TicTacToe {
        &self.0
    }
}

impl DerefMut for TicTacToe {
    fn deref_mut(&mut self) -> &mut tic_tac_toe_board::TicTacToe {
        &mut self.0
    }
}

impl TwoPlayerGame for TicTacToe {
    type Move = CellIndex;

    fn state<'a>(&self, moves_buf: &'a mut Vec<Self::Move>) -> GameState<'a, CellIndex> {
        moves_buf.clear();
        match self.0.state() {
            TicTacToeState::VictoryPlayerOne => GameState::WinPlayerOne,
            TicTacToeState::VictoryPlayerTwo => GameState::WinPlayerTwo,
            TicTacToeState::Draw => GameState::Draw,
            TicTacToeState::TurnPlayerOne | TicTacToeState::TurnPlayerTwo => {
                moves_buf.extend(self.0.open_fields());
                GameState::Moves(&moves_buf[..])
            }
        }
    }

    fn play(&mut self, mv: &CellIndex) {
        self.0.play_move(mv);
    }

    fn current_player(&self) -> Player {
        match self.0.state() {
            TicTacToeState::TurnPlayerOne | TicTacToeState::VictoryPlayerTwo => Player::One,
            TicTacToeState::TurnPlayerTwo
            | TicTacToeState::VictoryPlayerOne
            | TicTacToeState::Draw => Player::Two,
        }
    }
}
