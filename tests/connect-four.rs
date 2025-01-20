use std::fmt::{self, Display};

use connect_four_solver::{Column, Solver};
use monte_carlo_tree_search::{simulation, Count, GameState, TwoPlayerGame};
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
            wins_player_one: 5,
            wins_player_two: 0,
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
            let num_playouts = 1_000;
            let tree = Tree::with_playouts(ConnectFour(game), num_playouts, &mut rng);
            tree.children
                .iter()
                .max_by(|(child_a, _), (child_b, _)| {
                    let a = child_a.as_ref().unwrap().score.score(0);
                    let b = child_b.as_ref().unwrap().score.score(0);
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

#[derive(Debug)]
pub struct Tree<G: TwoPlayerGame, S: Search> {
    score: S::NodeState,
    children: Vec<(Option<Self>, G::Move)>,
}

impl<G, S> Tree<G, S>
where
    G: TwoPlayerGame,
    S: Search,
{
    pub fn new(moves: impl Iterator<Item = G::Move>, bias: S::NodeState) -> Self {
        Self {
            score: bias,
            children: moves.map(|move_| (None, move_)).collect(),
        }
    }
}

impl<G> Tree<G, Uct>
where
    G: TwoPlayerGame,
{
    pub fn with_playouts(game: G, num_playouts: u32, rng: &mut impl Rng) -> Self {
        let mut moves_buf = Vec::new();
        game.state(&mut moves_buf);
        let bias = simulation(game.clone(), rng);
        let mut tree = Self::new(moves_buf.iter().cloned(), bias);
        for _ in 1..num_playouts {
            tree.playout(game.clone(), rng);
        }
        tree
    }

    /// Playout one cycle of selection, expansion, simulation and backpropagation.
    pub fn playout(&mut self, root_game: G, rng: &mut impl Rng) {
        let mut game = root_game.clone();
        let (mut path, selected) = self.select_leaf(&mut game);

        let bias = if let Some((next_move, bias)) = selected.expand(game, rng) {
            path.push(next_move);
            bias
        } else {
            simulation(root_game, rng)
        };

        self.backpropagation(&path, bias);
    }

    /// Selects a leaf of the tree.
    ///
    /// # Return
    ///
    /// The path from the root to the selected leaf.
    pub fn select_leaf(&mut self, game: &mut G) -> (Vec<G::Move>, &mut Self) {
        let mut current = self;
        let mut path = Vec::new();
        while !current.is_leaf() {
            let (child, move_) = current
                .children
                .iter_mut()
                .max_by(|a, b| {
                    let a =
                        a.0.as_ref()
                            .unwrap()
                            .score
                            .ucb(current.score.total() as f32, game.current_player());
                    let b =
                        b.0.as_ref()
                            .unwrap()
                            .score
                            .ucb(current.score.total() as f32, game.current_player());
                    a.partial_cmp(&b).unwrap()
                })
                .expect("Children must not be empty");
            path.push(move_.clone());
            current = child.as_mut().expect("Child must be Some");
        }
        (path, current)
    }

    pub fn expand(
        &mut self,
        mut game: G,
        rng: &mut impl Rng,
    ) -> Option<(G::Move, Count)> {
        let mut candidates: Vec<_> = self
            .children
            .iter_mut()
            .filter(|(tree, _column)| tree.is_none())
            .collect();
        if let Some((child, move_)) = candidates.choose_mut(rng) {
            game.play(move_);
            let mut moves = Vec::new();
            game.state(&mut moves);
            let bias = simulation(game, rng);
            *child = Some(Tree::new(moves.into_iter(), bias));
            Some((move_.clone(), bias))
        } else {
            // Selected child has been in a terminal state
            None
        }
    }

    /// A leaf is any node with no children or unexplored children
    pub fn is_leaf(&self) -> bool {
        self.children.iter().any(|(child, _)| child.is_none()) || self.children.is_empty()
    }

    pub fn backpropagation(&mut self, path: &[G::Move], score: Count) {
        let mut current = self;
        current.score += score;
        for move_ in path {
            let (child, _) = current
                .children
                .iter_mut()
                .find(|(_, m)| m == move_)
                .expect("Child must exist");
            current = child.as_mut().expect("Child must be Some");
            current.score += score;
        }
    }
}

pub trait Search {
    type NodeState;

    fn bias(board: &impl TwoPlayerGame, rng: &mut impl Rng) -> Self::NodeState;
}

/// **U**pper **c**onfidence bound for **t**rees.
struct Uct;

impl Search for Uct {
    type NodeState = Count;

    fn bias(board: &impl TwoPlayerGame, rng: &mut impl Rng) -> Count {
        simulation(board.clone(), rng)
    }
}
