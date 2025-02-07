use std::cmp::Ordering;

use rand::{seq::IndexedRandom as _, Rng};

use crate::{
    tree::{Link, Node, Tree},
    Evaluation, Player, Policy, TwoPlayerGame,
};

/// A tree there the nodes represent game states and the links represent moves. The tree does only
/// store the root game state and reconstruct the nodes based on the moves. It does store an
/// evaluation for each node though. The evaluation is updated during each playout.
pub struct Search<G: TwoPlayerGame, P: Policy<G>> {
    /// Game state of the root node.
    game: G,
    tree: Tree<P::Evaluation, G::Move>,
    /// A buffer we use to store moves during node expansion. We have this as a member to avoid
    /// repeated allocation.
    move_buf: Vec<G::Move>,
    /// In order to choose a child node to expand at random, we (re)use this buffer in order to
    /// avoid its repeated allocation.
    candidate_link_index_buf: Vec<usize>,
    /// Remember the best move from the root node. Only change this move if we find a better one.
    /// This is different from just picking one of the best moves, as we would not replace the best
    /// move with one that is just as good. The reason for this is that our evaluation does only
    /// distinguish between win, draw and loss, but not contain any information about how far the
    /// loss is away, or how many errors the opponent could make. However the win we can achieve the
    /// earliest, is likely the best play a human would choose, and on the other hand, the position
    /// we would take the longest to realize it is a loose, is likely the one which allows the
    /// opponent to make the most mistakes.
    best_link: Option<usize>,
    /// Controls selection, evaluation and backpropagation.
    policy: P,
}

impl<G, P> Search<G, P>
where
    G: TwoPlayerGame,
    P: Policy<G>,
{
    pub fn new(game: G, policy: P) -> Self {
        let mut move_buf = Vec::new();
        let estimated_outcome = P::Evaluation::init_from_game_state(&game.state(&mut move_buf));
        let root = Node::new(usize::MAX, 0, move_buf.len(), estimated_outcome);
        let nodes = vec![root];
        let links: Vec<_> = move_buf
            .drain(..)
            .map(|move_| Link {
                child: usize::MAX,
                move_,
            })
            .collect();
        // Choose the first move as the best move to start, only so, that if [`Self::best_move`] is
        // called, before the first playout, it will return a move and not `None`.
        let best_link = if links.is_empty() { None } else { Some(0) };
        let tree = Tree { nodes, links };
        Self {
            game,
            tree,
            move_buf,
            candidate_link_index_buf: Vec::new(),
            best_link,
            policy,
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
        if self.tree.nodes[0].evaluation.is_solved() {
            return false;
        }

        let Selection {
            node_index: to_be_expanded_index,
            board: mut game,
            has_unexplored_children,
        } = self.select_unexplored_node();

        if !has_unexplored_children {
            return false;
        }

        // Create a new child node for the selected node and let `game` represent its state
        let new_node_index = self.expand(to_be_expanded_index, &mut game, rng);

        // Player whom gets to choose the next turn in the board the (new) leaf node represents.
        let player = game.current_player();

        // If the game is not in a terminal state, start a simulation to gain an initial estimate
        if !self.tree.nodes[new_node_index].evaluation.is_solved() {
            let bias = self.policy.bias(game, rng);
            self.tree.nodes[new_node_index].evaluation = bias;
        }

        self.backpropagation(new_node_index, player);
        self.update_best_link();
        true
    }

    /// Picks one of the best moves for the current player. `None` if the root node has no children.
    pub fn best_move(&self) -> Option<G::Move> {
        self.best_link
            .map(|link_index| self.tree.links[link_index].move_)
    }

    pub fn num_nodes(&self) -> usize {
        self.tree.nodes.len()
    }

    pub fn num_links(&self) -> usize {
        self.tree.links.len()
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    /// Count of playouts of the root node.
    pub fn evaluation(&self) -> P::Evaluation {
        self.tree.nodes[0].evaluation
    }

    pub fn eval_by_move(&self) -> impl Iterator<Item = (G::Move, P::Evaluation)> + '_ {
        self.tree.child_links(0)
            .map(move |link| {
                if link.is_explored() {
                    let child = &self.tree.nodes[link.child];
                    (link.move_, child.evaluation)
                } else {
                    (link.move_, self.policy.unexplored_bias())
                }
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
                .filter(|link| !self.tree.nodes[link.child].evaluation.is_solved())
                .max_by(|a, b| {
                    let selecting_player = game.current_player();
                    let a = self.tree.nodes[a.child].evaluation.selection_weight(
                        &self.tree.nodes[current_node_index].evaluation,
                        selecting_player,
                    );
                    let b = self.tree.nodes[b.child].evaluation.selection_weight(
                        &self.tree.nodes[current_node_index].evaluation,
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
        let link_index = self.pick_unexplored_child_of(to_be_expanded_index, rng);
        let link = &mut self.tree.links[link_index];

        game.play(&link.move_);
        let new_node_game_state = game.state(&mut self.move_buf);
        // Index there new node is created
        let new_node_index = self.tree.nodes.len();
        link.child = new_node_index;
        let grandchildren_begin = self.tree.links.len();
        let grandchildren_end = grandchildren_begin + new_node_game_state.moves().len();
        let eval = P::Evaluation::init_from_game_state(&new_node_game_state);
        self.tree
            .links
            .extend(self.move_buf.drain(..).map(|move_| Link {
                child: usize::MAX,
                move_,
            }));

        self.tree.nodes.push(Node::new(
            to_be_expanded_index,
            grandchildren_begin,
            grandchildren_end,
            eval,
        ));
        new_node_index
    }

    fn backpropagation(&mut self, node_index: usize, mut player: Player) {
        let mut delta = self.tree.nodes[node_index].evaluation.initial_delta();
        let mut current_child_index = node_index;
        let mut maybe_current_index = self.tree.nodes[node_index].parent_index();
        while let Some(current_node_index) = maybe_current_index {
            player.flip();

            let mut current_evaluation = self.tree.nodes[current_node_index].evaluation;
            delta = current_evaluation.update(
                self.sibling_evalutations(current_node_index, current_child_index),
                delta,
                player,
            );
            let node = &mut self.tree.nodes[current_node_index];
            current_child_index = current_node_index;
            node.evaluation = current_evaluation;
            maybe_current_index = node.parent_index();
        }
    }

    /// All evaluations of the siblings of the given child node. If a sibling is not yet explored,
    /// the evaluation will be `None`.
    ///
    /// # Parameters
    ///
    /// * `parent_index` - Parent of all the siblings and the child
    /// * `child_index` - Index of the child node. Must be a child of the node pointed to by
    ///   `parent_index`. The child will excluded from the items of the iterator.
    fn sibling_evalutations(
        &self,
        parent_index: usize,
        child_index: usize,
    ) -> impl Iterator<Item = Option<P::Evaluation>> + '_ {
        self.tree.child_links(parent_index).filter_map(move |link| {
            if link.is_explored() {
                if link.child == child_index {
                    None
                } else {
                    Some(Some(self.tree.nodes[link.child].evaluation))
                }
            } else {
                Some(None)
            }
        })
    }

    fn update_best_link(&mut self) {
        let current_player = self.game.current_player();
        let root = &self.tree.nodes[0];
        for link_index in root.children_begin..root.children_end {
            if self.best_link.is_none() {
                self.best_link = Some(link_index);
                continue;
            }
            let candidate_evaluation = self.evaluation_by_link_index(link_index);
            let best_so_far_evaluation = self.evaluation_by_link_index(self.best_link.unwrap());
            if candidate_evaluation.cmp_for(&best_so_far_evaluation, current_player)
                == Ordering::Greater
            {
                self.best_link = Some(link_index);
            }
        }
    }

    /// Evaluation of a node the link directs to. Handles unexplored nodes.
    fn evaluation_by_link_index(&self, link_index: usize) -> P::Evaluation {
        let link = self.tree.links[link_index];
        if link.is_explored() {
            self.tree.nodes[link.child].evaluation
        } else {
            self.policy.unexplored_bias()
        }
    }

    /// Link index of a random unexplored child of the selected node.
    fn pick_unexplored_child_of(&mut self, node_index: usize, rng: &mut impl Rng) -> usize {
        let node = &self.tree.nodes[node_index];
        let child_links_indices = node.children_begin..node.children_end;
        self.candidate_link_index_buf.clear();
        self.candidate_link_index_buf.extend(
            child_links_indices.filter(|&link_index| !self.tree.links[link_index].is_explored()),
        );
        self.candidate_link_index_buf
            .choose(rng)
            .copied()
            .expect("To be expandend node must have unexplored children")
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
