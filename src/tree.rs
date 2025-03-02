pub const ROOT_INDEX: usize = 0;

/// A tree data structure. Its nodes can have arbitrary numbers of children. At the moment a node is
/// added the number of its potential children must be known. The nodes these links are pointing to
/// can be added later though. We call this "expansion".
pub struct Tree<N, L> {
    /// We store all the nodes of the tree in a vector to avoid allocations. We refer to the nodes
    /// using indices.
    nodes: Vec<Node<N>>,
    /// Since we want to support Nodes with arbitrary many links, we store the links in their own
    /// vector. Each node will have a range in this vector indicated by a start and end index. We
    /// use usize::Max to indicate, that the node is not expanded yet.
    links: Vec<Link<L>>,
}

impl<N, L> Tree<N, L>
where
    L: Copy,
{
    /// Creates a new tree with a root node.
    pub fn new(initial_root_payload: N, links: impl Iterator<Item = L>) -> Self {
        let links: Vec<_> = links
            .map(|move_| Link {
                child: usize::MAX,
                move_,
            })
            .collect();
        let root = Node::new(0, links.len(), initial_root_payload);
        Self {
            nodes: vec![root],
            links,
        }
    }

    /// Links to the children of the node
    pub fn children(&self, node_index: usize) -> impl ExactSizeIterator<Item = Link<L>> + '_ {
        let node = &self.nodes[node_index];
        self.links[node.children_begin..node.children_end]
            .iter()
            .copied()
    }

    /// The move which would lead to the child node. In case the child node is already existing we
    /// also get the evalutation, or `None` otherwise.
    pub fn child_move_and_eval(
        &self,
        node_index: usize,
    ) -> impl ExactSizeIterator<Item = (L, Option<&N>)> + '_
    where
        L: Copy,
    {
        self.children(node_index).map(|link| {
            if link.is_explored() {
                (link.move_, Some(&self.nodes[link.child].evaluation))
            } else {
                (link.move_, None)
            }
        })
    }

    /// Evaluation of the node identified by `node_index`.
    pub fn evaluation(&self, node_index: usize) -> &N {
        &self.nodes[node_index].evaluation
    }

    /// Evaluation of the node identified by `node_index`.
    pub fn evaluation_mut(&mut self, node_index: usize) -> &mut N {
        &mut self.nodes[node_index].evaluation
    }

    pub fn add(
        &mut self,
        parent_index: usize,
        child_number: usize,
        payload: N,
        links: impl Iterator<Item = L>,
    ) -> usize {
        let children_begin = self.links.len();
        self.links.extend(links.map(|move_| Link {
            child: usize::MAX,
            move_,
        }));
        let children_end = self.links.len();
        let node = Node::new(children_begin, children_end, payload);
        let node_index = self.nodes.len();
        let link_index = self.nodes[parent_index].children_begin + child_number;
        self.links[link_index].child = node_index;
        self.nodes.push(node);
        node_index
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn num_links(&self) -> usize {
        self.links.len()
    }

    /// All evaluations of the siblings of the given child node. If a sibling is not yet explored,
    /// the evaluation will be `None`.
    ///
    /// # Parameters
    ///
    /// * `parent_index` - Parent of all the siblings and the child
    /// * `child_index` - Index of the child node. Must be a child of the node pointed to by
    /// * `parent_index`. The child will excluded from the items of the iterator.
    pub fn sibling_evalutations(
        &self,
        parent_index: usize,
        child_index: usize,
    ) -> impl Iterator<Item = Option<&N>> + '_ {
        self.children(parent_index).filter_map(move |link| {
            if link.is_explored() {
                if link.child == child_index {
                    None
                } else {
                    Some(Some(&self.nodes[link.child].evaluation))
                }
            } else {
                Some(None)
            }
        })
    }
}

#[derive(Debug)]
struct Node<E> {
    /// Index into `Tree::links` where the children of this node start. `0` if the node does not
    /// have children.
    children_begin: usize,
    /// Index into `Tree::links` where the children of this node end, or more precise, the children
    /// of the next node would start, i.e. `children_begin + num_children`. `0` if the node does not
    /// have children.
    children_end: usize,
    evaluation: E,
}

impl<E> Node<E> {
    pub fn new(children_begin: usize, children_end: usize, estimated_outcome: E) -> Self {
        Self {
            children_begin,
            children_end,
            evaluation: estimated_outcome,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Link<M> {
    /// Index of the child node. `usize::MAX` if the node is not expanded yet.
    pub child: usize,
    /// Move that lead to the child node, from the board state of the parent node.
    pub move_: M,
}

impl<M> Link<M> {
    pub fn is_explored(&self) -> bool {
        self.child != usize::MAX
    }
}
