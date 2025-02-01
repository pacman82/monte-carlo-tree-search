use std::cmp::Ordering;

use rand::{seq::IndexedRandom as _, Rng};

use crate::{evaluation::CountWithDecided, Bias, Count, Evaluation, Player, TwoPlayerGame};

/// A tree there the nodes represent game states and the links represent moves. The tree does only
/// store the root game state and reconstruct the nodes based on the moves. It does store an
/// evaluation for each node though. The evaluation is updated during each playout.
pub struct Tree<G: TwoPlayerGame, B> {
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
    /// Used to get an initial estimate of the outcome of a new node.
    bias: B,
}

impl<G, B> Tree<G, B>
where
    G: TwoPlayerGame,
    B: Bias<G>,
{
    pub fn new(game: G, bias: B) -> Self {
        let mut move_buf = Vec::new();
        let estimated_outcome = game.state(&mut move_buf).map_to_evaluation();
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
        Self {
            game,
            nodes,
            links,
            move_buf,
            candidate_link_index_buf: Vec::new(),
            best_link,
            bias,
        }
    }

    pub fn with_playouts(game: G, bias: B, num_playouts: u32, rng: &mut impl Rng) -> Self {
        let mut tree = Self::new(game, bias);
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
        let Some((to_be_expanded_index, mut game)) = self.select_unexplored_node() else {
            return false;
        };
        // Create a new child node for the selected node and let `game` represent its state
        let new_node_index = self.expand(to_be_expanded_index, &mut game, rng);

        // Player whom gets to choose the next turn in the board the (new) leaf node represents.
        let player = game.current_player();

        // If the game is not in a terminal state, start a simulation to gain an initial estimate
        if !self.nodes[new_node_index].evaluation.is_solved() {
            let bias = self.bias.bias(game, &mut self.move_buf, rng);
            self.nodes[new_node_index].evaluation = bias;
        }

        self.backpropagation(new_node_index, player);
        self.update_best_link();
        true
    }

    fn update_best_link(&mut self) {
        let current_player = self.game.current_player();
        let root = &self.nodes[0];
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
    fn evaluation_by_link_index(&self, link_index: usize) -> CountWithDecided {
        let link = self.links[link_index];
        if link.is_explored() {
            self.nodes[link.child].evaluation
        } else {
            CountWithDecided::default()
        }
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
    fn select_unexplored_node(&self) -> Option<(usize, G)> {
        let mut current_node_index = 0;
        let mut game = self.game.clone();
        while !self.has_unexplored_children(current_node_index) {
            let Some(best_ucb) = self
                .child_links(current_node_index)
                // Filter all solved positions. We may assume link is explored, because of the
                // entry condition of the while loop
                .filter(|link| !self.nodes[link.child].evaluation.is_solved())
                .max_by(|a, b| {
                    let selecting_player = game.current_player();
                    let a = self.nodes[a.child].evaluation.selection_weight(
                        &self.nodes[current_node_index].evaluation,
                        selecting_player,
                    );
                    let b = self.nodes[b.child].evaluation.selection_weight(
                        &self.nodes[current_node_index].evaluation,
                        selecting_player,
                    );
                    a.partial_cmp(&b).unwrap()
                })
            else {
                // We should never decent into a solved node. Any unsolved node should have at least
                // one unsolved child, otherwise, would it not have been solved during
                // backpropagation?
                debug_assert_eq!(current_node_index, 0);
                return None;
            };
            game.play(&best_ucb.move_);
            current_node_index = best_ucb.child;
        }
        Some((current_node_index, game))
    }

    /// Link index of a random unexplored child of the selected node.
    fn pick_unexplored_child_of(&mut self, node_index: usize, rng: &mut impl Rng) -> usize {
        let node = &self.nodes[node_index];
        let child_links_indices = node.children_begin..node.children_end;
        self.candidate_link_index_buf.clear();
        self.candidate_link_index_buf.extend(
            child_links_indices.filter(|&link_index| !self.links[link_index].is_explored()),
        );
        self.candidate_link_index_buf
            .choose(rng)
            .copied()
            .expect("To be expandend node must have unexplored children")
    }

    /// Expand an unexplored child of the selected node. Mutates `game` to represent state of the
    /// node indicated by the retunned index.
    ///
    /// # Return
    ///
    /// Index of newly created child node.
    fn expand(&mut self, to_be_expanded_index: usize, game: &mut G, rng: &mut impl Rng) -> usize {
        let link_index = self.pick_unexplored_child_of(to_be_expanded_index, rng);
        let link = &mut self.links[link_index];

        game.play(&link.move_);
        let new_node_game_state = game.state(&mut self.move_buf);
        // Index there new node is created
        let new_node_index = self.nodes.len();
        link.child = new_node_index;
        let grandchildren_begin = self.links.len();
        let grandchildren_end = grandchildren_begin + new_node_game_state.moves().len();
        let eval = new_node_game_state.map_to_evaluation();
        self.links.extend(self.move_buf.drain(..).map(|move_| Link {
            child: usize::MAX,
            move_,
        }));

        self.nodes.push(Node::new(
            to_be_expanded_index,
            grandchildren_begin,
            grandchildren_end,
            eval,
        ));
        new_node_index
    }

    fn backpropagation(&mut self, node_index: usize, mut player: Player) {
        let mut delta = self.nodes[node_index].evaluation;
        let mut current = self.nodes[node_index].parent_index();
        // Total of child node before propagation. The original node index is the newly added leaf
        // so we can assume it to be 1. We keep track of this value going upwards, in case an
        // a solved node flips to an undecided node, to count all previous visits to the solved
        // child node as one the analogos of the solved state.
        let mut child_count = delta.into_count();
        while let Some(current_node_index) = current {
            player.flip();

            let (updated_evaluation, new_delta) = self.updated_evaluation(
                current_node_index,
                self.child_evalutations(current_node_index),
                delta,
                player,
                child_count,
            );
            delta = new_delta;
            let node = &mut self.nodes[current_node_index];
            child_count = node.evaluation.into_count();
            node.evaluation = updated_evaluation;
            current = node.parent_index();
        }
    }

    /// All evaluations of the children of the given node. If a child is not yet explored, the
    /// evaluation will be `None`.
    fn child_evalutations(
        &self,
        node_index: usize,
    ) -> impl Iterator<Item = Option<CountWithDecided>> + '_ {
        self.child_links(node_index).map(move |link| {
            if link.is_explored() {
                Some(self.nodes[link.child].evaluation)
            } else {
                None
            }
        })
    }

    /// Update the evaluation of a node with the propagated evaluation.
    ///
    /// # Return
    ///
    /// First element is the evaluation of the node specified in node_index. The second element is
    /// the delta which should be propagated to its parent node. How can these differ? Usually the
    /// two are identical, but consider a situation in which we learn that a node is a proofen loss
    /// for the choosing player given perfect play of both players. Yet all of its siblings are
    /// draws. In such a situation we would propagate the draw, but still asign the loss to the
    /// loosing node.
    fn updated_evaluation(
        &self,
        node_index: usize,
        child_evaluations: impl Iterator<Item = Option<CountWithDecided>>,
        propagated_evaluation: CountWithDecided,
        choosing_player: Player,
        previous_child_count: Count,
    ) -> (CountWithDecided, CountWithDecided) {
        let old_evaluation = self.nodes[node_index].evaluation;
        if propagated_evaluation == CountWithDecided::Win(choosing_player) {
            // If it is the choosing players turn, she will choose a win
            return (propagated_evaluation, propagated_evaluation);
        }
        // If the choosing player is not guaranteed to win let's check if there is a draw or a loss
        let loss = CountWithDecided::Win(choosing_player.opponent());
        if propagated_evaluation.is_solved() {
            let mut acc = Some(loss);
            for maybe_eval in child_evaluations {
                let Some(child_eval) = maybe_eval else {
                    // Still has unexplored children, so we can not be sure the current node is a
                    // draw or a loss.
                    acc = None;
                    break;
                };
                if child_eval == CountWithDecided::Draw {
                    // Found a draw, so we can be sure its not a loss
                    acc = Some(CountWithDecided::Draw);
                } else if child_eval != loss {
                    // Found a child neither draw or loss, so we can not rule out a victory yet
                    acc = None;
                    break;
                }
            }
            if let Some(evaluation) = acc {
                return (evaluation, evaluation);
            }
        }
        // No deterministic outcome, let's propagete the counts
        let propageted_count = match propagated_evaluation {
            CountWithDecided::Win(Player::One) => {
                let mut count = Count {
                    wins_player_one: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountWithDecided::Win(Player::Two) => {
                let mut count = Count {
                    wins_player_two: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountWithDecided::Draw => {
                let mut count = Count {
                    draws: previous_child_count.total() + propagated_evaluation.total(),
                    ..Default::default()
                };
                count -= previous_child_count;
                count
            }
            CountWithDecided::Undecided(count) => count,
        };

        match old_evaluation {
            CountWithDecided::Undecided(mut count) => {
                count += propageted_count;
                (
                    CountWithDecided::Undecided(count),
                    CountWithDecided::Undecided(propageted_count),
                )
            }
            _ => (
                old_evaluation,
                CountWithDecided::Undecided(propageted_count),
            ),
        }
    }

    /// `true` if the node has at least one child which is not explored yet.
    fn has_unexplored_children(&self, node_index: usize) -> bool {
        let mut it = self.child_links(node_index);
        it.any(|link| !link.is_explored())
    }

    fn child_links(&self, node_index: usize) -> impl ExactSizeIterator<Item = Link<G::Move>> + '_ {
        let node = &self.nodes[node_index];
        self.links[node.children_begin..node.children_end]
            .iter()
            .copied()
    }
}

impl<G, B> Tree<G, B>
where
    G: TwoPlayerGame,
{
    /// Count of playouts of the root node.
    pub fn evaluation(&self) -> CountWithDecided {
        self.nodes[0].evaluation
    }

    pub fn eval_by_move(&self) -> impl Iterator<Item = (G::Move, CountWithDecided)> + '_ {
        let root = &self.nodes[0];
        self.links[root.children_begin..root.children_end]
            .iter()
            .map(move |link| {
                if link.is_explored() {
                    let child = &self.nodes[link.child];
                    (link.move_, child.evaluation)
                } else {
                    (link.move_, CountWithDecided::default())
                }
            })
    }

    /// Picks one of the best movesfor the current player. `None` if the root node has no children.
    pub fn best_move(&self) -> Option<G::Move> {
        self.best_link
            .map(|link_index| self.links[link_index].move_)
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn num_links(&self) -> usize {
        self.links.len()
    }

    pub fn game(&self) -> &G {
        &self.game
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
    evaluation: CountWithDecided,
}

impl Node {
    fn new(
        parent: usize,
        children_begin: usize,
        children_end: usize,
        estimated_outcome: CountWithDecided,
    ) -> Self {
        Self {
            parent,
            children_begin,
            children_end,
            evaluation: estimated_outcome,
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
