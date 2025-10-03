use crate::ids::PoolId;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct World<S> {
    pub pool_states: HashMap<PoolId, S>,
}
