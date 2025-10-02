use crate::ids::{PoolId, TokenId};
use alloy_primitives::U256;

pub trait Pool {
    type State: Clone;
    fn id(&self) -> PoolId;
    fn supports(&self, from: TokenId, to: TokenId) -> bool;
    fn swap(&self, st: &mut Self::State, from: TokenId, to: TokenId, amt_in: U256) -> U256;
}
