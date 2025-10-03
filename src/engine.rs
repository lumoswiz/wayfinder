use crate::{
    Pool, World,
    ids::{PoolId, TokenId},
};
use alloy_primitives::U256;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Step {
    pub pool: PoolId,
    pub from: TokenId,
    pub to: TokenId,
    pub amt_in: U256,
    pub amt_out: U256,
}

#[derive(Clone, Debug)]
pub struct Path {
    pub steps: Vec<Step>,
}

pub type Hop = (PoolId, TokenId, TokenId);

pub struct Engine<'a, P: Pool> {
    pub pools: &'a HashMap<PoolId, P>,
}

impl<'a, P: Pool> Engine<'a, P> {
    pub fn new(pools: &'a HashMap<PoolId, P>) -> Self {
        Self { pools }
    }

    pub fn simulate_chained(&self, world: &World<P::State>, plan: &[Hop], first_in: U256) -> Path {
        assert!(!plan.is_empty(), "path must have at least one hop");

        let start_token = plan[0].1;
        let mut amt_in = first_in;

        let mut scratch: HashMap<PoolId, P::State> = HashMap::new();

        let mut last_token = start_token;
        let mut steps = Vec::with_capacity(plan.len());

        for &(pid, from, to) in plan {
            assert_eq!(
                from, last_token,
                "path discontinuity: expected from {:?}",
                last_token
            );

            let pool = self.pools.get(&pid).expect("missing pool impl");
            debug_assert!(pool.supports(from, to), "unsupported direction");

            let st = scratch.entry(pid).or_insert_with(|| {
                world
                    .pool_states
                    .get(&pid)
                    .expect("missing pool state")
                    .clone()
            });

            let amt_out = if amt_in.is_zero() {
                U256::ZERO
            } else {
                pool.swap(st, from, to, amt_in)
            };

            steps.push(Step {
                pool: pid,
                from,
                to,
                amt_in,
                amt_out,
            });

            last_token = to;
            amt_in = amt_out;
        }

        Path { steps }
    }
}
