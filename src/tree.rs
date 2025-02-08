/// A tree data structure. Its nodes can have arbitrary numbers of children. At the moment a node is
/// added the number of its potential children must be known. The nodes these links are pointing to
/// can be added later though. We call this "expansion".
pub struct Tree<N, L> {
    /// We store all the nodes of the tree in a vector to avoid allocations. We refer to the nodes
    /// using indices.
    pub nodes: Vec<Node<N>>,
    /// Since we want to support Nodes with arbitrary many links, we store the links in their own
    /// vector. Each node will have a range in this vector indicated by a start and end index. We
    /// use usize::Max to indicate, that the node is not expanded yet.
    pub links: Vec<Link<L>>,
}

impl<N, L> Tree<N, L> {
    /// Creates a new tree with a root node.
    pub fn new(initial_root_payload: N, links: impl Iterator<Item = L>) -> Self {
        let links: Vec<_> = links
            .map(|move_| Link {
                child: usize::MAX,
                move_,
            })
            .collect();
        let root = Node::new(usize::MAX, 0, links.len(), initial_root_payload);
        Self {
            nodes: vec![root],
            links,
        }
    }

    /// Links to the children of the node
    pub fn child_links(&self, node_index: usize) -> impl ExactSizeIterator<Item = Link<L>> + '_
    where
        L: Copy,
    {
        let node = &self.nodes[node_index];
        self.links[node.children_begin..node.children_end]
            .iter()
            .copied()
    }
}

#[derive(Debug)]
pub struct Node<E> {
    /// Index of the parent node. The root node will be set to `usize::MAX`.
    pub parent: usize,
    /// Index into `Tree::links` where the children of this node start. `0` if the node does not
    /// have children.
    pub children_begin: usize,
    /// Index into `Tree::links` where the children of this node end, or more precise, the children
    /// of the next node would start, i.e. `children_begin + num_children`. `0` if the node does not
    /// have children.
    pub children_end: usize,
    pub evaluation: E,
}

impl<E> Node<E> {
    pub fn new(
        parent: usize,
        children_begin: usize,
        children_end: usize,
        estimated_outcome: E,
    ) -> Self {
        Self {
            parent,
            children_begin,
            children_end,
            evaluation: estimated_outcome,
        }
    }

    pub fn parent_index(&self) -> Option<usize> {
        (self.parent != usize::MAX).then_some(self.parent)
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
