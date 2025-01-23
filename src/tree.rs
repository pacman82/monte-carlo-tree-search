use rand::{seq::SliceRandom as _, Rng};

use crate::{count::EstimatedOutcome, simulation, Count, GameState, TwoPlayerGame};

pub struct Tree<G: TwoPlayerGame> {
    /// Game state of the root node.
    game: G,
    /// We store all the nodes of the tree in a vector to avoid allocations. We refer to the nodes
    /// using indices.
    nodes: Vec<Node>,
    /// Since we want to support Nodes with arbitrary many links, we store the links in their own
    /// vector. Each node will have a range in this vector indicated by a start and end index. We
    /// use usize::Max to indicate, that the node is not expanded yet.
    links: Vec<Link<G::Move>>,
    /// A buffer we use to store moves during node expansion. We have this as a member to avoid
    /// repeated allocation.
    move_buf: Vec<G::Move>,
}

impl<G> Tree<G>
where
    G: TwoPlayerGame,
{
    pub fn new(game: G) -> Self {
        let mut move_buf = Vec::new();
        let estimated_outcome = match game.state(&mut move_buf) {
            GameState::Moves(_) | GameState::Draw => EstimatedOutcome::Undecided(Count::default()),
            GameState::WinPlayerOne => EstimatedOutcome::WinPlayerOne,
            GameState::WinPlayerTwo => EstimatedOutcome::WinPlayerTwo,
        };
        let root = Node::new(usize::MAX, 0, move_buf.len(), estimated_outcome);
        let nodes = vec![root];
        let links = move_buf
            .drain(..)
            .map(|move_| Link {
                child: usize::MAX,
                move_,
            })
            .collect();
        Self {
            game,
            nodes,
            links,
            move_buf,
        }
    }

    pub fn with_playouts(game: G, num_playouts: u32, rng: &mut impl Rng) -> Self {
        let mut tree = Self::new(game);
        for _ in 0..num_playouts {
            tree.playout(rng);
        }
        tree
    }

    /// Playout one cycle of selection, expansion, simulation and backpropagation.
    pub fn playout(&mut self, rng: &mut impl Rng) {
        let (leaf_index, mut game) = self.select_leaf();
        let expanded_index = self.expand(leaf_index, &mut game, rng);
        let count = simulation(game, &mut self.move_buf, rng);
        self.backpropagation(expanded_index, count);
    }

    /// Count of playouts of the root node.
    pub fn estimate_outcome(&self) -> EstimatedOutcome {
        self.nodes[0].estimated_outcome
    }

    pub fn estimated_outcome_by_move(
        &self,
    ) -> impl Iterator<Item = (G::Move, EstimatedOutcome)> + '_ {
        let root = &self.nodes[0];
        self.links[root.children_begin..root.children_end]
            .iter()
            .map(move |link| {
                let child = &self.nodes[link.child];
                (link.move_, child.estimated_outcome)
            })
    }

    /// Picks a move with the highest reward for the current player. `None` if the root node has no
    /// children.
    pub fn best_move(&self) -> Option<G::Move> {
        let current_player = self.game.current_player();
        let root = &self.nodes[0];
        self.links[root.children_begin..root.children_end]
            .iter()
            .map(|link| {
                let eo = if link.is_explored() {
                    self.nodes[link.child].estimated_outcome.reward(current_player)
                } else {
                    // Use default constructed estimated outcome if node is not explored yet.
                    EstimatedOutcome::default().reward(current_player)
                };
                (link.move_, eo)
            })
            .max_by(|(_move_a, reward_a), (_move_b, reward_b)| {
                reward_a.partial_cmp(reward_b).expect("Reward must be comparable")
            })
            .map(|(move_, _reward)| move_)
    }

    /// Selects a leaf of the tree.
    ///
    /// # Return
    ///
    /// Index of a leaf and the game state it represents.
    fn select_leaf(&self) -> (usize, G) {
        let mut current_node_index = 0;
        let mut game = self.game.clone();
        while !self.is_leaf(current_node_index) {
            let best_ucb = self
                .children(current_node_index)
                .max_by(|a, b| {
                    let a = self.nodes[a.child].estimated_outcome.selection_weight(
                        self.nodes[current_node_index].estimated_outcome.total() as f32,
                        game.current_player(),
                    );
                    let b = self.nodes[b.child].estimated_outcome.selection_weight(
                        self.nodes[current_node_index].estimated_outcome.total() as f32,
                        game.current_player(),
                    );
                    a.partial_cmp(&b).unwrap()
                })
                .expect("Children must not be empty");
            game.play(&best_ucb.move_);
            current_node_index = best_ucb.child;
        }
        (current_node_index, game)
    }

    /// Expand an unexplored child of the selected node. Mutates `game` to represent expanded child.
    ///
    /// # Return
    ///
    /// Index of newly created child node.
    fn expand(&mut self, selected_node_index: usize, game: &mut G, rng: &mut impl Rng) -> usize {
        let selected_node = &self.nodes[selected_node_index];
        let children = &mut self.links[selected_node.children_begin..selected_node.children_end];
        let mut candidates: Vec<_> = children
            .iter_mut()
            .filter(|link| !link.is_explored())
            .collect();
        if let Some(link) = candidates.choose_mut(rng) {
            game.play(&link.move_);
            game.state(&mut self.move_buf);

            let new_node_index = self.nodes.len();
            link.child = new_node_index;
            self.nodes.push(Node::new(
                selected_node_index,
                self.links.len(),
                self.links.len() + self.move_buf.len(),
                EstimatedOutcome::default(),
            ));
            self.links.extend(self.move_buf.drain(..).map(|move_| Link {
                child: usize::MAX,
                move_,
            }));
            new_node_index
        } else {
            // Selected child has no unexplored children => since it is a leaf, it must be in a
            // terminal state
            selected_node_index
        }
    }

    fn backpropagation(&mut self, node_index: usize, count: EstimatedOutcome) {
        let mut current = Some(node_index);
        while let Some(node_index) = current {
            let node = &mut self.nodes[node_index];
            node.estimated_outcome.propagate_child(count);
            current = node.parent_index();
        }
    }

    /// A leaf is any node with no children or unexplored children
    fn is_leaf(&self, node_index: usize) -> bool {
        let mut it = self.children(node_index);
        if it.len() == 0 {
            return true;
        }
        it.any(|link| !link.is_explored())
    }

    fn children(&self, node_index: usize) -> impl ExactSizeIterator<Item = Link<G::Move>> + '_ {
        let node = &self.nodes[node_index];
        self.links[node.children_begin..node.children_end]
            .iter()
            .copied()
    }
}

#[derive(Debug)]
struct Node {
    /// Index of the parent node. The root node will be set to `usize::MAX`.
    parent: usize,
    /// Index into `Tree::links` where the children of this node start. `0` if the node does not
    /// have children.
    children_begin: usize,
    /// Index into `Tree::links` where the children of this node end, or more precise, the children
    /// of the next node would start, i.e. `children_begin + num_children`. `0` if the node does not
    /// have children.
    children_end: usize,
    estimated_outcome: EstimatedOutcome,
}

impl Node {
    fn new(parent: usize, children_begin: usize, children_end: usize, estimated_outcome: EstimatedOutcome) -> Self {
        Self {
            parent,
            children_begin,
            children_end,
            estimated_outcome,
        }
    }

    fn parent_index(&self) -> Option<usize> {
        (self.parent != usize::MAX).then_some(self.parent)
    }
}

#[derive(Debug, Clone, Copy)]
struct Link<M> {
    /// Index of the child node. `usize::MAX` if the node is not expanded yet.
    child: usize,
    /// Move that lead to the child node, from the board state of the parent node.
    move_: M,
}

impl<M> Link<M> {
    fn is_explored(&self) -> bool {
        self.child != usize::MAX
    }
}
