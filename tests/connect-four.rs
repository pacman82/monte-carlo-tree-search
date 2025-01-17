use monte_carlo_tree_search::Count;
use connect_four_solver::{Column, ConnectFour, Solver};
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

    let mut game = ConnectFour::new();
    let mut solver = Solver::new();
    let mut moves = Vec::new();

    while !game.is_over() {
        let next_move = if game.stones() % 2 == 0 {
            let num_playouts = 100;
            let tree = Tree::with_playouts(game, num_playouts, &mut rng);
            tree.children.iter().max_by(|(child_a, _), (child_b, _)| {
                let a = child_a.as_ref().unwrap().score.score();
                let b = child_b.as_ref().unwrap().score.score();
                a.partial_cmp(&b).unwrap()
            }).unwrap().1
        } else {
            solver.best_moves(&game, &mut moves);
            *moves.choose(&mut rng).unwrap()
        };
        eprintln!("column: {next_move}");
        game.play(next_move);
        eprintln!("{game}");
    }

}

/// Play random moves, until the game is over and report the score
pub fn simulation(mut game: ConnectFour, rng: &mut impl Rng) -> Count {
    let stones_begin = game.stones();
    while !game.is_over() {
        let candidates: Vec<_> = game.legal_moves().collect();
        let selected_move = *candidates.choose(rng).unwrap();
        game.play(selected_move);
    }
    let mut score = Count::default();
    if game.is_victory() {
        if game.stones() % 2 == stones_begin % 2 {
            score.wins_other_player = 1;
        } else {
            score.wins_current_player = 1;
        }
    } else {
        score.draws = 1;
    }
    score
}

#[derive(Debug)]
pub struct Tree {
    score: Count,
    children: Vec<(Option<Tree>, Column)>,
}

impl Tree {
    pub fn new(game: ConnectFour) -> Self {
        let children = if game.is_over() {
            Vec::new()
        } else {
            game.legal_moves().map(|move_| (None, move_)).collect()
        };
        Self {
            score: Count::default(),
            children,
        }
    }

    pub fn with_playouts(game: ConnectFour, num_playouts: u32, rng: &mut impl Rng) -> Self {
        let mut tree = Self::new(game);
        for _ in 0..num_playouts {
            tree.playout(game, rng);
        }
        tree
    }

    /// Playout one cycle of selection, expansion, simulation and backpropagation.
    pub fn playout(&mut self, root_game: ConnectFour, rng: &mut impl Rng) {
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
    pub fn select_leaf(&self) -> Vec<Column> {
        let mut current = self;
        let mut path = Vec::new();
        while !current.is_leaf() {
            let (child, move_) = current
                .children
                .iter().max_by(|a, b| {
                    let a = a.0.as_ref().unwrap().score.ucb(current.score.total() as f32, 1.0f32);
                    let b = b.0.as_ref().unwrap().score.ucb(current.score.total() as f32, 1.0f32);
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
        path: &[Column],
        game: &mut ConnectFour,
        rng: &mut impl Rng,
    ) -> Option<Column> {
        let mut current = self;
        for move_ in path {
            let (child, _move) = current
                .children
                .iter_mut()
                .find(|(_, m)| m == move_)
                .expect("Child must exist");
            game.play(*move_);
            current = child.as_mut().expect("Child must be Some");
        }
        let mut candidates: Vec<_> = current
            .children
            .iter_mut()
            .filter(|(tree, _column)| tree.is_none())
            .collect();
        if let Some((child, move_)) = candidates.choose_mut(rng) {
            game.play(*move_);
            *child = Some(Tree::new(*game));
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

    pub fn backpropagation(&mut self, path: &[Column], mut score: Count) {
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
