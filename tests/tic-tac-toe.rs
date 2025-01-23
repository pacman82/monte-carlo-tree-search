use std::ops::{Deref, DerefMut};

use monte_carlo_tree_search::{EstimatedOutcome, GameState, Tree, TwoPlayerGame};
use rand::{rngs::StdRng, SeedableRng as _};
use tic_tac_toe_board::{CellIndex, TicTacToeState};

#[test]
fn play_tic_tac_toe() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut game = TicTacToe::new();

    let num_playouts = 1_000;
    while !game.0.state().is_terminal() {
        let tree = Tree::with_playouts(game, num_playouts, &mut rng);
        let best_move = tree.best_move().unwrap();
        game.play_move(&best_move);
        // game.print_to(stderr()).unwrap();
        // eprintln!();
    }

    let mut moves_buf = Vec::new();
    assert_eq!(GameState::Draw, game.state(&mut moves_buf));
}

#[test]
fn prevent_immediate_win_of_other_player() {
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
    // game.print_to(stderr()).unwrap();

    let num_playouts = 100;
    let tree = Tree::with_playouts(game, num_playouts, &mut rng);
    let counts = tree.estimated_outcome_by_move().collect::<Vec<_>>();
    for (mv, count) in counts {
        eprintln!(
            "Move: {:?} Count: {:?}, Reward: {}",
            mv,
            count,
            count.reward(0)
        );
    }
    assert_eq!(CellIndex::new(7), tree.best_move().unwrap());
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
    let tree = Tree::with_playouts(game, num_playouts, &mut rng);

    assert_eq!(EstimatedOutcome::WinPlayerOne, tree.estimate_outcome())
}

#[test]
fn suggest_winning_move() {
    let mut rng = StdRng::seed_from_u64(42);
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



    let num_playouts = 1;
    let tree = Tree::with_playouts(game, num_playouts, &mut rng);

    assert_eq!(CellIndex::new(3), tree.best_move().unwrap())
}

/// With few or zero playouts, we can be in a situation, there not all nodes of the root are
/// explored. We want to handle unexplored direct children of the root node, withouth panic.
#[test]
fn unexplored_root_childs() {
    let game = TicTacToe::new();
    
    let tree = Tree::new(game);

    assert!(tree.best_move().is_some())
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

    fn current_player(&self) -> u8 {
        match self.0.state() {
            TicTacToeState::TurnPlayerOne
            | TicTacToeState::VictoryPlayerTwo
            | TicTacToeState::Draw => 0,
            TicTacToeState::TurnPlayerTwo | TicTacToeState::VictoryPlayerOne => 1,
        }
    }
}
