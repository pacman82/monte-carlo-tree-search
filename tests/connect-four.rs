use std::fmt::{self, Display};

use connect_four_solver::{Column, Solver};
use monte_carlo_tree_search::{Count, GameState, TwoPlayerGame};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

#[test]
fn play_move_connect_four() {
    let mut rng = StdRng::seed_from_u64(42);
    let game = ConnectFour::new();
    let num_playouts = 100;

    let tree = Tree::with_playouts(game, num_playouts, &mut rng);

    for (child, move_) in &tree.children {
        eprintln!(
            "Score child {:?}: {:?}",
            move_,
            child.as_ref().map(|c| c.score)
        );
    }
}

#[test]
fn start_from_terminal_position() {
    let mut rng = StdRng::seed_from_u64(42);

    // First player has won
    let game = ConnectFour::from_move_list("1212121");
    let num_playouts = 5;
    let tree = Tree::with_playouts(game, num_playouts, &mut rng);

    assert_eq!(
        Count {
            wins_current_player: 0,
            wins_other_player: 5,
            draws: 0
        },
        tree.score
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
            let num_playouts = 100;
            let tree = Tree::with_playouts(ConnectFour(game), num_playouts, &mut rng);
            tree.children
                .iter()
                .max_by(|(child_a, _), (child_b, _)| {
                    let a = child_a.as_ref().unwrap().score.score();
                    let b = child_b.as_ref().unwrap().score.score();
                    a.partial_cmp(&b).unwrap()
                })
                .unwrap()
                .1
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
            // is from the perspective of the player which played the last stone, theras `GameState`
            // is from the current players perspective
            return GameState::Loss;
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

/// Play random moves, until the game is over and report the score
pub fn simulation(mut game: impl TwoPlayerGame, rng: &mut impl Rng) -> Count {
    let start_player = game.current_player();
    let mut moves_buf = Vec::new();
    loop {
        match game.state(&mut moves_buf) {
            GameState::Moves(legal_moves) => {
                let selected_move = legal_moves.choose(rng).unwrap();
                game.play(selected_move)
            }
            GameState::Win => {
                break Count {
                    wins_current_player: (start_player == game.current_player()) as u32,
                    wins_other_player: (start_player != game.current_player()) as u32,
                    draws: 0,
                }
            }
            GameState::Loss => {
                break Count {
                    wins_current_player: (start_player != game.current_player()) as u32,
                    wins_other_player: (start_player == game.current_player()) as u32,
                    draws: 0,
                }
            }
            GameState::Draw => {
                break Count {
                    wins_current_player: 0,
                    wins_other_player: 0,
                    draws: 1,
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Tree<Move> {
    score: Count,
    children: Vec<(Option<Self>, Move)>,
}

impl<Move> Tree<Move>
where
    Move: Copy + Eq,
{
    pub fn new(game: impl TwoPlayerGame<Move = Move>) -> Self {
        let mut moves_buf = Vec::new();
        game.state(&mut moves_buf);
        Self {
            score: Count::default(),
            children: moves_buf.into_iter().map(|move_| (None, move_)).collect(),
        }
    }

    pub fn with_playouts(
        game: impl TwoPlayerGame<Move = Move>,
        num_playouts: u32,
        rng: &mut impl Rng,
    ) -> Self {
        let mut tree = Self::new(game.clone());
        for _ in 0..num_playouts {
            tree.playout(game.clone(), rng);
        }
        tree
    }

    /// Playout one cycle of selection, expansion, simulation and backpropagation.
    pub fn playout(&mut self, root_game: impl TwoPlayerGame<Move = Move>, rng: &mut impl Rng) {
        let mut path = self.select_leaf();

        let mut simulated_game = root_game;
        if let Some(next_move) = self.expand(&path, &mut simulated_game, rng) {
            path.push(next_move);
        }

        let score = simulation(simulated_game, rng);
        self.backpropagation(&path, score);
    }

    /// Selects a leaf of the tree.
    ///
    /// # Return
    ///
    /// The path from the root to the selected leaf.
    pub fn select_leaf(&self) -> Vec<Move> {
        let mut current = self;
        let mut path = Vec::new();
        while !current.is_leaf() {
            let (child, move_) = current
                .children
                .iter()
                .max_by(|a, b| {
                    let a =
                        a.0.as_ref()
                            .unwrap()
                            .score
                            .ucb(current.score.total() as f32);
                    let b =
                        b.0.as_ref()
                            .unwrap()
                            .score
                            .ucb(current.score.total() as f32);
                    a.partial_cmp(&b).unwrap()
                })
                .expect("Children must not be empty");
            path.push(*move_);
            current = child.as_ref().expect("Child must be Some");
        }
        path
    }

    pub fn expand(
        &mut self,
        path: &[Move],
        game: &mut impl TwoPlayerGame<Move = Move>,
        rng: &mut impl Rng,
    ) -> Option<Move> {
        let mut current = self;
        for move_ in path {
            let (child, _move) = current
                .children
                .iter_mut()
                .find(|(_, m)| m == move_)
                .expect("Child must exist");
            game.play(move_);
            current = child.as_mut().expect("Child must be Some");
        }
        let mut candidates: Vec<_> = current
            .children
            .iter_mut()
            .filter(|(tree, _column)| tree.is_none())
            .collect();
        if let Some((child, move_)) = candidates.choose_mut(rng) {
            game.play(move_);
            *child = Some(Tree::new(game.clone()));
            Some(*move_)
        } else {
            // Selected child has been in a terminal state
            None
        }
    }

    /// A leaf is any node with on children or unexplored children
    pub fn is_leaf(&self) -> bool {
        self.children.iter().any(|(child, _)| child.is_none()) || self.children.is_empty()
    }

    pub fn backpropagation(&mut self, path: &[Move], mut score: Count) {
        let mut current = self;
        current.score += score;
        if path.len() % 2 == 0 {
            score.flip_players();
        }
        for move_ in path {
            let (child, _) = current
                .children
                .iter_mut()
                .find(|(_, m)| m == move_)
                .expect("Child must exist");
            current = child.as_mut().expect("Child must be Some");
            score.flip_players();
            current.score += score;
        }
    }
}
