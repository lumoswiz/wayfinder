use crate::ids::{PoolId, TokenId};
use alloy_primitives::Address;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct TokenMeta {
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Clone, Debug)]
pub enum PoolKind {
    UniV3,
}

#[derive(Clone, Debug)]
pub struct PoolMeta {
    pub address: Address,
    pub kind: PoolKind,
    pub token0: TokenId,
    pub token1: TokenId,
    pub fee: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Registry {
    pub token_meta: HashMap<TokenId, TokenMeta>,
    pub pool_meta: HashMap<PoolId, PoolMeta>,
    pub token_by_addr: HashMap<Address, TokenId>,
    pub pool_by_addr: HashMap<Address, PoolId>,
}

impl Registry {
    pub fn upsert_token(&mut self, tid: TokenId, meta: TokenMeta) {
        self.token_by_addr.insert(meta.address, tid);
        self.token_meta.insert(tid, meta);
    }

    pub fn upsert_pool(&mut self, pid: PoolId, meta: PoolMeta) {
        self.pool_by_addr.insert(meta.address, pid);
        self.pool_meta.insert(pid, meta);
    }

    pub fn token(&self, tid: TokenId) -> Option<&TokenMeta> {
        self.token_meta.get(&tid)
    }

    pub fn pool(&self, pid: PoolId) -> Option<&PoolMeta> {
        self.pool_meta.get(&pid)
    }
}
