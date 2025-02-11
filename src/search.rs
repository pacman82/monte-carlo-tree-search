use std::cmp::Ordering;

use rand::{seq::IndexedRandom as _, Rng};

use crate::{
    tree::{Tree, ROOT_INDEX},
    Evaluation, Explorer, Player, TwoPlayerGame,
};

/// A tree there the nodes represent game states and the links represent moves. The tree does only
/// store the root game state and reconstruct the nodes based on the moves. It does store an
/// evaluation for each node though. The evaluation is updated during each playout.
pub struct Search<G: TwoPlayerGame, P: Explorer<G>> {
    /// Game state of the root node.
    game: G,
    tree: Tree<P::Evaluation, G::Move>,
    /// A buffer we use to store moves during node expansion. We have this as a member to avoid
    /// repeated allocation.
    move_buf: Vec<G::Move>,
    /// In order to choose a child node to expand at random, we (re)use this buffer in order to
    /// avoid its repeated allocation.
    candidate_children_buf: Vec<(G::Move, usize)>,
    /// Remember the best move from the root node. Only change this move if we find a better one.
    /// This is different from just picking one of the best moves, as we would not replace the best
    /// move with one that is just as good. The reason for this is that our evaluation does only
    /// distinguish between win, draw and loss, but not contain any information about how far the
    /// loss is away, or how many errors the opponent could make. However the win we can achieve the
    /// earliest, is likely the best play a human would choose, and on the other hand, the position
    /// we would take the longest to realize it is a loose, is likely the one which allows the
    /// opponent to make the most mistakes.
    best_move: Option<G::Move>,
    /// Controls selection, evaluation and backpropagation.
    policy: P,
    
}

impl<G, P> Search<G, P>
where
    G: TwoPlayerGame,
    P: Explorer<G>,
{
    pub fn new(game: G, policy: P) -> Self {
        let mut move_buf = Vec::new();
        let estimated_outcome = P::Evaluation::init_from_game_state(&game.state(&mut move_buf));
        let tree = Tree::new(estimated_outcome, move_buf.drain(..));
        // Choose the first move as the best move to start, only so, that if [`Self::best_move`] is
        // called, before the first playout, it will return a move and not `None`.
        let best_move = tree.child_move_and_eval(ROOT_INDEX).next().map(|(move_, _)| move_);
        Self {
            game,
            tree,
            move_buf,
            candidate_children_buf: Vec::new(),
            policy,
            best_move,
        }
    }

    pub fn with_playouts(game: G, policy: P, num_playouts: u32, rng: &mut impl Rng) -> Self {
        let mut tree = Self::new(game, policy);
        for _ in 0..num_playouts {
            if !tree.playout(rng) {
                break;
            }
        }
        tree
    }

    /// Playout one cycle of selection, expansion, simulation and backpropagation. `true` if the
    /// playout may have changed the evaluation of the root, `false` if the game is already solved.
    pub fn playout(&mut self, rng: &mut impl Rng) -> bool {
        if self.tree.evaluation(ROOT_INDEX).is_solved() {
            return false;
        }

        let Selection {
            node_index,
            board: mut game,
            has_unexplored_children,
        } = self.select_unexplored_node();

        let (player, node_index, delta) = if has_unexplored_children {
            // Create a new child node for the selected node and let `game` represent its state
            let new_node_index = self.expand(node_index, &mut game, rng);

            // Player whom gets to choose the next turn in the board the (new) leaf node represents.
            let player = game.current_player();

            // If the game is not in a terminal state, start a simulation to gain an initial estimate
            if !self.tree.evaluation(new_node_index).is_solved() {
                let bias = self.policy.bias(game, rng);
                *self.tree.evaluation_mut(new_node_index) = bias;
            }
            let delta = self
                .policy
                .initial_delta(self.tree.evaluation(new_node_index));
            (player, new_node_index, delta)
        } else {
            // Existing node
            let player = game.current_player();
            let delta = self
                .policy
                .reevaluate(game, self.tree.evaluation_mut(node_index));
            (player, node_index, delta)
        };

        self.backpropagation(node_index, delta, player);
        self.update_best_move();
        true
    }

    /// Picks one of the best moves for the current player. `None` if the root node has no children.
    pub fn best_move(&self) -> Option<G::Move> {
        self.best_move
    }

    pub fn num_nodes(&self) -> usize {
        self.tree.num_nodes()
    }

    pub fn num_links(&self) -> usize {
        self.tree.num_links()
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    /// Count of playouts of the root node.
    pub fn evaluation(&self) -> P::Evaluation {
        *self.tree.evaluation(ROOT_INDEX)
    }

    pub fn eval_by_move(&self) -> impl ExactSizeIterator<Item = (G::Move, P::Evaluation)> + '_ {
        self.tree
            .child_move_and_eval(ROOT_INDEX)
            .map(|(move_, maybe_eval)| {
                (
                    move_,
                    maybe_eval.copied().unwrap_or(self.policy.unexplored_bias()),
                )
            })
    }

    /// Selects a node of the tree of which is not solved (yet?). This means we do not know given
    /// perfect play, if the game would result in win, loose or draw starting from the nodes
    /// position. Since the resulting node is unsolved. In addition to being unsolved, we also
    /// demand that the node is unexplored, i.e. it has at least one link, which is not yet directed
    /// at a node. As such the node returned by this method is suitable for expansion.
    ///
    /// # Return
    ///
    /// Index of the selected leaf node and the game state of the node.
    fn select_unexplored_node(&self) -> Selection<G> {
        let mut current_node_index = 0;
        let mut game = self.game.clone();
        while !self.has_unexplored_children(current_node_index) {
            let Some(best_ucb) = self
                .tree
                .child_links(current_node_index)
                // Filter all solved positions. We may assume link is explored, because of the
                // entry condition of the while loop
                .filter(|link| !self.tree.evaluation(link.child).is_solved())
                .max_by(|a, b| {
                    let selecting_player = game.current_player();
                    let a = self.tree.evaluation(a.child).selection_weight(
                        self.tree.evaluation(current_node_index),
                        selecting_player,
                    );
                    let b = self.tree.evaluation(b.child).selection_weight(
                        self.tree.evaluation(current_node_index),
                        selecting_player,
                    );
                    a.partial_cmp(&b).unwrap()
                })
            else {
                return Selection {
                    node_index: current_node_index,
                    board: game,
                    has_unexplored_children: false,
                };
            };
            game.play(&best_ucb.move_);
            current_node_index = best_ucb.child;
        }
        Selection {
            node_index: current_node_index,
            board: game,
            has_unexplored_children: true,
        }
    }

    /// Expand an unexplored child of the selected node. Mutates `game` to represent state of the
    /// node indicated by the retunned index.
    ///
    /// # Return
    ///
    /// Index of newly created child node.
    fn expand(&mut self, to_be_expanded_index: usize, game: &mut G, rng: &mut impl Rng) -> usize {
        self.candidate_children_buf.clear();
        self.candidate_children_buf.extend(
            self.tree.child_move_and_eval(to_be_expanded_index).enumerate().filter_map(|(i, (move_, eval))| {
                if eval.is_none() {
                    Some((move_, i))
                } else {
                    None
                }
            }),
        );
        let (move_, child_n) = self.candidate_children_buf.choose(rng).unwrap();

        game.play(move_);
        let new_node_game_state = game.state(&mut self.move_buf);
        let eval = P::Evaluation::init_from_game_state(&new_node_game_state);
        let new_node_index = self
            .tree
            .add(to_be_expanded_index, *child_n, eval, self.move_buf.drain(..));
        new_node_index
    }

    fn backpropagation(&mut self, node_index: usize, mut delta: P::Delta, mut player: Player) {
        let mut current_child_index = node_index;
        let mut maybe_current_index = self.tree.parent_index(node_index);
        while let Some(current_node_index) = maybe_current_index {
            player.flip();

            let mut current_evaluation = *self.tree.evaluation(current_node_index);
            delta = self.policy.update(
                &mut current_evaluation,
                self.tree
                    .sibling_evalutations(current_node_index, current_child_index)
                    .map(|e| e.copied()),
                delta,
                player,
            );
            current_child_index = current_node_index;
            *self.tree.evaluation_mut(current_node_index) = current_evaluation;
            maybe_current_index = self.tree.parent_index(current_node_index);
        }
    }

    fn update_best_move(&mut self) {
        let current_player = self.game.current_player();
        let unexplored_bias = self.policy.unexplored_bias();
        // `true` if a evaluates to better or equal than b
        let cmp_eval = |a: Option<&P::Evaluation>, b: Option<&P::Evaluation>|{
            let a = a.unwrap_or(&unexplored_bias);
            let b = b.unwrap_or(&unexplored_bias);
            a.cmp_for(b, current_player)
        };

        let mut best_eval = None;
        let mut best_move = None;
        for (move_, eval) in self.tree.child_move_and_eval(ROOT_INDEX) {
            // First pass through loop
            if best_move.is_none() {
                best_eval = eval;
                best_move = Some(move_);
                continue;
            }
            let cmp = cmp_eval(eval, best_eval);
            let should_replace_best = if move_ == self.best_move().unwrap() { cmp != Ordering::Less } else { cmp == Ordering::Greater };
            if should_replace_best {
                best_move = Some(move_);
                best_eval = eval;
            }
        }
        self.best_move = best_move;        
    }

    /// `true` if the node has at least one child which is not explored yet.
    fn has_unexplored_children(&self, node_index: usize) -> bool {
        let mut it = self.tree.child_links(node_index);
        it.any(|link| !link.is_explored())
    }
}

/// Result of [`Tree::select_unexplored_node`]. Provides the input for expansion and
/// backpropagation. We need to distinguish between a node we want to expand or a node we without
/// selectable children, which is reevaluated.
struct Selection<G> {
    /// Index of the selected node
    node_index: usize,
    /// A board representing the game state associated with the selected node.
    board: G,
    /// `true` if the node has at least one child, which in unexplored and suitable for expansion.
    /// `false` if the node is either terminal, solved or both. It does not have any children which
    /// would be considered during a selection phase.
    has_unexplored_children: bool,
}
