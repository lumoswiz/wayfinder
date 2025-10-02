use crate::ids::{PoolId, TokenId};
use petgraph::Direction;
use petgraph::prelude::*;
use petgraph::stable_graph::StableDiGraph;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum NodeKind {
    Token(TokenId),
    Pool(PoolId),
}

pub struct AMMGraph {
    pub g: StableDiGraph<NodeKind, ()>,
    pub token_idx: HashMap<TokenId, NodeIndex>,
    pub pool_idx: HashMap<PoolId, NodeIndex>,
}

impl AMMGraph {
    pub fn new() -> Self {
        Self {
            g: StableDiGraph::new(),
            token_idx: HashMap::new(),
            pool_idx: HashMap::new(),
        }
    }

    pub fn add_token(&mut self, id: TokenId) -> NodeIndex {
        *self
            .token_idx
            .entry(id)
            .or_insert_with(|| self.g.add_node(NodeKind::Token(id)))
    }

    pub fn add_pool(&mut self, id: PoolId) -> NodeIndex {
        *self
            .pool_idx
            .entry(id)
            .or_insert_with(|| self.g.add_node(NodeKind::Pool(id)))
    }

    pub fn connect_token_to_pool(&mut self, t: TokenId, p: PoolId) {
        let tix = self.add_token(t);
        let pix = self.add_pool(p);
        self.g.add_edge(tix, pix, ());
    }

    pub fn connect_pool_to_token(&mut self, p: PoolId, t: TokenId) {
        let pix = self.add_pool(p);
        let tix = self.add_token(t);
        self.g.add_edge(pix, tix, ());
    }

    pub fn pools_accepting(&self, t: TokenId) -> impl Iterator<Item = NodeIndex> + '_ {
        let tix = self.token_idx[&t];
        self.g
            .neighbors_directed(tix, Direction::Outgoing)
            .filter(|&n| matches!(self.g[n], NodeKind::Pool(_)))
    }

    pub fn tokens_emitted_by(&self, p: PoolId) -> impl Iterator<Item = NodeIndex> + '_ {
        let pix = self.pool_idx[&p];
        self.g
            .neighbors_directed(pix, Direction::Outgoing)
            .filter(|&n| matches!(self.g[n], NodeKind::Token(_)))
    }

    fn add_edge_unique(&mut self, from: NodeIndex, to: NodeIndex) {
        if self.g.find_edge(from, to).is_none() {
            self.g.add_edge(from, to, ());
        }
    }

    pub fn connect_bidirectional_pair(&mut self, p: PoolId, a: TokenId, b: TokenId) {
        let aix = self.add_token(a);
        let bix = self.add_token(b);
        let pix = self.add_pool(p);

        self.add_edge_unique(aix, pix);
        self.add_edge_unique(pix, bix);

        self.add_edge_unique(bix, pix);
        self.add_edge_unique(pix, aix);
    }
}
