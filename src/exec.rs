use crate::{
    Pool, World,
    ids::{PoolId, TokenId},
};
use alloy_primitives::U256;
use std::collections::HashMap;

pub fn apply_swap<P: Pool>(
    pool_impls: &HashMap<PoolId, P>,
    world: &mut World<P::State>,
    pid: PoolId,
    from: TokenId,
    to: TokenId,
    amt_in: U256,
) -> U256 {
    let pool = pool_impls.get(&pid).expect("missing pool impl");
    debug_assert!(pool.supports(from, to), "unsupported direction");

    let from_bal = world.holdings.entry(from).or_default();
    if from_bal.is_zero() {
        return U256::ZERO;
    }
    let use_in = (*from_bal).min(amt_in);
    *from_bal -= use_in;

    let out = {
        let st = world.pool_states.get_mut(&pid).expect("missing pool state");
        pool.swap(st, from, to, use_in)
    };

    *world.holdings.entry(to).or_default() += out;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
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
        fn supports(&self, from: TokenId, to: TokenId) -> bool {
            (from == self.a && to == self.b) || (from == self.b && to == self.a)
        }
        fn swap(&self, _st: &mut Self::State, _from: TokenId, _to: TokenId, amt_in: U256) -> U256 {
            let num = U256::from(10_000u32 - self.fee_bps);
            (amt_in * num) / U256::from(10_000u32)
        }
    }

    #[test]
    fn single_hop_apply_swap_debits_credits() {
        let x = TokenId(1);
        let y = TokenId(2);
        let p = PoolId(10);
        let impls = HashMap::from([(
            p,
            FeePool {
                id_: p,
                a: x,
                b: y,
                fee_bps: 100,
            },
        )]);
        let mut world: World<UnitState> = World::default();
        world.holdings.insert(x, U256::from(1_000_000u64));
        world.pool_states.insert(p, UnitState::default());

        let out = apply_swap(&impls, &mut world, p, x, y, U256::from(250_000u64));
        assert_eq!(out, U256::from(247_500u64));
        assert_eq!(world.holdings.get(&x), Some(&U256::from(750_000u64)));
        assert_eq!(world.holdings.get(&y), Some(&U256::from(247_500u64)));
    }
}
