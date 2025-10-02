use crate::{
    Pool, World,
    ids::{PoolId, TokenId},
};
use alloy_primitives::U256;
use std::{cmp::min, collections::HashMap};

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

pub struct Engine<'a, P: Pool> {
    pub pools: &'a HashMap<PoolId, P>,
}

impl<'a, P: Pool> Engine<'a, P> {
    pub fn new(pools: &'a HashMap<PoolId, P>) -> Self {
        Self { pools }
    }

    pub fn apply_swap(
        &self,
        world: &mut World<P::State>,
        pid: PoolId,
        from: TokenId,
        to: TokenId,
        amt_in: U256,
    ) -> U256 {
        let pool = self.pools.get(&pid).expect("missing pool impl");
        debug_assert!(pool.supports(from, to), "unsupported direction");

        let from_bal = world.holdings.entry(from).or_default();
        if from_bal.is_zero() {
            return U256::ZERO;
        }
        let use_in = min(*from_bal, amt_in);
        *from_bal -= use_in;

        let out = {
            let st = world.pool_states.get_mut(&pid).expect("missing pool state");
            pool.swap(st, from, to, use_in)
        };

        *world.holdings.entry(to).or_default() += out;
        out
    }

    pub fn execute_path(
        &self,
        world: &mut World<P::State>,
        path_spec: &[(PoolId, TokenId, TokenId, U256)],
    ) -> Path {
        assert!(!path_spec.is_empty(), "path must have at least one hop");
        let mut steps = Vec::with_capacity(path_spec.len());
        let mut last_token = path_spec.first().unwrap().1;

        for &(pid, from, to, amt_in) in path_spec {
            assert_eq!(
                from, last_token,
                "path discontinuity: expected hop from {:?}",
                last_token
            );
            let out = self.apply_swap(world, pid, from, to, amt_in);
            steps.push(Step {
                pool: pid,
                from,
                to,
                amt_in,
                amt_out: out,
            });
            last_token = to;
        }

        Path { steps }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;
    use std::collections::HashMap;

    #[derive(Clone, Debug, Default)]
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
        fn supports(&self, f: TokenId, t: TokenId) -> bool {
            (f == self.a && t == self.b) || (f == self.b && t == self.a)
        }
        fn swap(&self, _st: &mut Self::State, _f: TokenId, _t: TokenId, amt_in: U256) -> U256 {
            let num = U256::from(10_000u32 - self.fee_bps);
            (amt_in * num) / U256::from(10_000u32)
        }
    }

    #[test]
    fn single_hop_apply_swap_debits_credits() {
        let x = TokenId(1);
        let y = TokenId(2);
        let p = PoolId(10);

        let mut pools: HashMap<PoolId, FeePool> = HashMap::new();
        pools.insert(
            p,
            FeePool {
                id_: p,
                a: x,
                b: y,
                fee_bps: 100,
            },
        );

        let engine = Engine::new(&pools);

        let mut world: World<UnitState> = World::default();
        world.holdings.insert(x, U256::from(1_000_000u64));
        world.pool_states.insert(p, UnitState::default());

        let out = engine.apply_swap(&mut world, p, x, y, U256::from(250_000u64));
        assert_eq!(out, U256::from(247_500u64));
        assert_eq!(world.holdings.get(&x), Some(&U256::from(750_000u64)));
        assert_eq!(world.holdings.get(&y), Some(&U256::from(247_500u64)));
    }

    #[test]
    fn two_hop_trace_and_balances() {
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

        let plan = [
            (p_xy, x, y, U256::from(250_000u64)),
            (p_yz, y, z, U256::from(200_000u64)),
        ];

        let path = engine.execute_path(&mut world, &plan);

        assert_eq!(path.steps.len(), 2);
        assert_eq!(path.steps[0].pool, p_xy);
        assert_eq!(path.steps[0].from, x);
        assert_eq!(path.steps[0].to, y);
        assert_eq!(path.steps[0].amt_in, U256::from(250_000u64));
        assert_eq!(path.steps[0].amt_out, U256::from(247_500u64));

        assert_eq!(path.steps[1].pool, p_yz);
        assert_eq!(path.steps[1].from, y);
        assert_eq!(path.steps[1].to, z);
        assert_eq!(path.steps[1].amt_in, U256::from(200_000u64));
        assert_eq!(path.steps[1].amt_out, U256::from(196_000u64));

        assert_eq!(world.holdings[&x], U256::from(750_000u64));
        assert_eq!(world.holdings[&y], U256::from(47_500u64));
        assert_eq!(world.holdings[&z], U256::from(196_000u64));
    }

    #[test]
    #[should_panic(expected = "at least one hop")]
    fn empty_path_panics() {
        let pools: HashMap<PoolId, FeePool> = HashMap::new();
        let engine = Engine::new(&pools);
        let mut world: World<UnitState> = World::default();
        let _ = engine.execute_path(&mut world, &[]);
    }

    #[test]
    #[should_panic(expected = "discontinuity")]
    fn path_discontinuity_panics() {
        let x = TokenId(1);
        let y = TokenId(2);
        let z = TokenId(3);
        let p = PoolId(10);

        let mut pools: HashMap<PoolId, FeePool> = HashMap::new();
        pools.insert(
            p,
            FeePool {
                id_: p,
                a: x,
                b: y,
                fee_bps: 0,
            },
        );

        let engine = Engine::new(&pools);

        let mut world: World<UnitState> = World::default();
        world.pool_states.insert(p, UnitState::default());
        world.holdings.insert(x, U256::from(1));

        let _ = engine.execute_path(
            &mut world,
            &[(p, x, y, U256::from(1)), (p, z, y, U256::from(1))],
        );
    }
}
