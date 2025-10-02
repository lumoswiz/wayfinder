use crate::ids::{PoolId, TokenId};
use alloy_primitives::U256;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct World<S> {
    pub holdings: HashMap<TokenId, U256>,
    pub pool_states: HashMap<PoolId, S>,
}
