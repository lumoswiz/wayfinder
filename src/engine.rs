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

    pub fn simulate_chained(
        &self,
        world: &World<P::State>,
        plan: &[Hop],
        first_in: U256,
        cap_first_hop: bool,
    ) -> Path {
        assert!(!plan.is_empty(), "path must have at least one hop");

        let start_token = plan[0].1;
        let mut amt_in = if cap_first_hop {
            let have = *world.holdings.get(&start_token).unwrap_or(&U256::ZERO);
            first_in.min(have)
        } else {
            first_in
        };

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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct UnitState;

    #[derive(Clone, Debug)]
    struct FeePool {
        id_: PoolId,
        a: TokenId,
        b: TokenId,
        fee_bps: u32,
    }

    impl Pool for FeePool {
        type State = UnitState;

        fn id(&self) -> PoolId {
            self.id_
        }

        fn supports(&self, from: TokenId, to: TokenId) -> bool {
            (from == self.a && to == self.b) || (from == self.b && to == self.a)
        }

        fn swap(&self, _st: &mut Self::State, _from: TokenId, _to: TokenId, amt_in: U256) -> U256 {
            let num = U256::from(10_000u32 - self.fee_bps);
            (amt_in * num) / U256::from(10_000u32)
        }
    }

    #[test]
    fn chained_simulation() {
        let x = TokenId(1);
        let y = TokenId(2);
        let z = TokenId(3);
        let p_xy = PoolId(10);
        let p_yz = PoolId(11);

        let mut pools: HashMap<PoolId, FeePool> = HashMap::new();
        pools.insert(
            p_xy,
            FeePool {
                id_: p_xy,
                a: x,
                b: y,
                fee_bps: 100,
            },
        );
        pools.insert(
            p_yz,
            FeePool {
                id_: p_yz,
                a: y,
                b: z,
                fee_bps: 200,
            },
        );

        let engine = Engine::new(&pools);

        let mut world: World<UnitState> = World::default();
        world.pool_states.insert(p_xy, UnitState::default());
        world.pool_states.insert(p_yz, UnitState::default());
        world.holdings.insert(x, U256::from(1_000_000u64));
        world.holdings.insert(y, U256::ZERO);
        world.holdings.insert(z, U256::ZERO);

        let world_before = world.clone();

        let plan: &[Hop] = &[(p_xy, x, y), (p_yz, y, z)];
        let first_in = U256::from(250_000u64);

        let path = engine.simulate_chained(&world, plan, first_in, true);

        assert_eq!(world.pool_states, world_before.pool_states);
        assert_eq!(world.holdings, world_before.holdings);

        assert_eq!(path.steps.len(), 2);
        assert_eq!(path.steps[0].amt_in, U256::from(250_000u64));
        assert_eq!(path.steps[0].amt_out, U256::from(247_500u64));
        assert_eq!(path.steps[1].amt_in, path.steps[0].amt_out);
        assert_eq!(path.steps[1].amt_out, U256::from(242_550u64));

        assert_eq!(path.steps[1].to, z);
    }
}
