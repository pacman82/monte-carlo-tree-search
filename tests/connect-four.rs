use std::{mem, ops::AddAssign};

use connect_four_solver::{Column, ConnectFour};
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
        Score {
            wins_current_player: 0,
            wins_other_player: 5,
            draws: 0
        },
        tree.score
    );
}

/// Play random moves, until the game is over and report the score
pub fn simulation(mut game: ConnectFour, rng: &mut impl Rng) -> Score {
    let stones_begin = game.stones();
    while !game.is_over() {
        let candidates: Vec<_> = game.legal_moves().collect();
        let selected_move = *candidates.choose(rng).unwrap();
        game.play(selected_move);
    }
    let mut score = Score::default();
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

/// Score relative to the current player.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Score {
    /// Number of wins for the player who is about to make the next turn
    pub wins_current_player: u32,
    /// Number of wins for the other player, who is waiting for his/her turn
    pub wins_other_player: u32,
    pub draws: u32,
}

impl Score {
    /// Assign a score of 1 for winning, 0 for loosing and 0.5 for a draw. Divided by the number of
    /// playouts. Zero playouts will result in a score of 0.
    pub fn accumulated(&self) -> f32 {
        (self.wins_current_player as f32 + self.draws as f32)
            / (self.wins_current_player + self.wins_other_player + self.draws) as f32
    }

    /// The score from the other players perspective
    fn flip_players(&mut self) {
        mem::swap(
            &mut self.wins_current_player,
            &mut self.wins_other_player,
        );
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, other: Self) {
        self.wins_current_player += other.wins_current_player;
        self.wins_other_player += other.wins_other_player;
        self.draws += other.draws;
    }
}

#[derive(Debug)]
pub struct Tree {
    score: Score,
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
            score: Score::default(),
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
        let mut path = self.select_leaf(rng);

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
    pub fn select_leaf(&self, rng: &mut impl Rng) -> Vec<Column> {
        let mut current = self;
        let mut path = Vec::new();
        while !current.is_leaf() {
            let (child, move_) = current
                .children
                .choose(rng)
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

    pub fn backpropagation(&mut self, path: &[Column], mut score: Score) {
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
