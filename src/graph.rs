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

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(g: &AMMGraph, ix: NodeIndex) -> &NodeKind {
        &g.g[ix]
    }

    #[test]
    fn add_token_and_pool_are_idempotent() {
        let mut g = AMMGraph::new();
        let t = TokenId(1);
        let p = PoolId(100);

        let t1 = g.add_token(t);
        let t2 = g.add_token(t);
        assert_eq!(t1, t2, "add_token should be idempotent");
        assert!(matches!(kind(&g, t1), NodeKind::Token(TokenId(1))));
        assert_eq!(g.g.node_count(), 1);

        let p1 = g.add_pool(p);
        let p2 = g.add_pool(p);
        assert_eq!(p1, p2, "add_pool should be idempotent");
        assert!(matches!(kind(&g, p1), NodeKind::Pool(PoolId(100))));
        assert_eq!(g.g.node_count(), 2);
    }

    #[test]
    fn connect_token_to_pool_and_query() {
        let mut g = AMMGraph::new();
        let t = TokenId(1);
        let p = PoolId(10);

        g.connect_token_to_pool(t, p);

        let pools: Vec<_> = g.pools_accepting(t).collect();
        assert_eq!(pools.len(), 1);
        assert!(matches!(g.g[pools[0]], NodeKind::Pool(PoolId(10))));

        let tokens: Vec<_> = g.tokens_emitted_by(p).collect();
        assert!(tokens.is_empty());

        assert_eq!(g.g.edge_count(), 1);
        let (tix, pix) = (g.token_idx[&t], g.pool_idx[&p]);
        assert!(g.g.find_edge(tix, pix).is_some());
        assert!(
            g.g.find_edge(pix, tix).is_none(),
            "no implicit reverse edge"
        );
    }

    #[test]
    fn connect_pool_to_token_and_query() {
        let mut g = AMMGraph::new();
        let t = TokenId(2);
        let p = PoolId(20);

        g.connect_pool_to_token(p, t);

        let tokens: Vec<_> = g.tokens_emitted_by(p).collect();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(g.g[tokens[0]], NodeKind::Token(TokenId(2))));

        let pools: Vec<_> = g.pools_accepting(t).collect();
        assert!(pools.is_empty());

        assert_eq!(g.g.edge_count(), 1);
        let (tix, pix) = (g.token_idx[&t], g.pool_idx[&p]);
        assert!(g.g.find_edge(pix, tix).is_some());
        assert!(g.g.find_edge(tix, pix).is_none());
    }

    #[test]
    fn connect_bidirectional_pair_wires_both_directions_without_dupes() {
        let mut g = AMMGraph::new();
        let a = TokenId(7);
        let b = TokenId(8);
        let p = PoolId(70);

        g.connect_bidirectional_pair(p, a, b);

        let (aix, bix, pix) = (g.token_idx[&a], g.token_idx[&b], g.pool_idx[&p]);
        assert!(g.g.find_edge(aix, pix).is_some());
        assert!(g.g.find_edge(pix, bix).is_some());
        assert!(g.g.find_edge(bix, pix).is_some());
        assert!(g.g.find_edge(pix, aix).is_some());
        assert_eq!(g.g.edge_count(), 4);

        g.connect_bidirectional_pair(p, a, b);
        assert_eq!(g.g.edge_count(), 4, "edges should be unique");
    }
}
