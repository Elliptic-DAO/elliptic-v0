use crate::Vault;
use crate::{CKBTC, TAL};
use candid::Principal;
use ic_base_types::PrincipalId;
use proptest::prop_assert;
use proptest::proptest;
use proptest::{
    collection::{btree_map, vec as pvec},
    prelude::{any, Strategy},
};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

fn arb_vault() -> impl Strategy<Value = Vault> {
    (arb_principal(), any::<u64>(), arb_amount()).prop_map(|(owner, borrowed_tal, ckbtc_margin)| {
        Vault {
            owner,
            borrowed_tal_amount: TAL::from(borrowed_tal),
            ckbtc_margin_amount: CKBTC::from(ckbtc_margin.max(1_000_000)),
            vault_id: 0,
        }
    })
}

fn arb_principal() -> impl Strategy<Value = Principal> {
    (pvec(any::<u8>(), 32)).prop_map(|pk| PrincipalId::new_self_authenticating(&pk).0)
}

fn arb_usd_amount() -> impl Strategy<Value = TAL> {
    arb_amount().prop_map(|a| TAL::from(a))
}

fn arb_amount() -> impl Strategy<Value = u64> {
    1..21_000_000_00_000_000_u64
}

fn arb_btc_amount() -> impl Strategy<Value = u64> {
    1..21_000_000_00_000_000_u64
}

fn vault_vec_to_map(vaults_vec: Vec<Vault>) -> BTreeMap<u64, Vault> {
    let mut vaults = BTreeMap::new();
    let mut counter: u64 = 0;
    for vault in vaults_vec {
        counter += 1;
        vaults.insert(
            counter,
            Vault {
                owner: vault.owner,
                borrowed_tal_amount: vault.borrowed_tal_amount,
                ckbtc_margin_amount: vault.ckbtc_margin_amount,
                vault_id: counter,
            },
        );
    }
    vaults
}

proptest! {
    #[test]
    fn proptest_distribute_accross_vaults(
        vaults_vec in pvec(arb_vault(), 1..10),
        target_borrowed_tal in any::<u64>(),
        target_ckbtc_margin in arb_amount(),
    ) {
        let vaults = vault_vec_to_map(vaults_vec);
        let sum_ckbtc_margin_amount: CKBTC = vaults.values().map(|vault| vault.ckbtc_margin_amount).sum();
        let ckbtc_margin = target_ckbtc_margin.max(1_000_000).min(sum_ckbtc_margin_amount.to_u64());
        let target_vault = Vault {
            owner: Principal::anonymous(),
            borrowed_tal_amount: TAL::from(target_borrowed_tal),
            ckbtc_margin_amount: CKBTC::from(ckbtc_margin),
            vault_id: vaults.last_key_value().unwrap().1.vault_id + 1,
        };

        // Ensure that the sum of ckbtc_margin in vaults is greater or equal to target_vault's ckbtc_margin
        prop_assert!(sum_ckbtc_margin_amount.to_u64() >= target_vault.ckbtc_margin_amount.to_u64());
        // Call the function
        let result = crate::state::distribute_accross_vaults(&vaults, target_vault);
        let tal_distributed: TAL = result.iter().map(|e| e.tal_share_amount).sum();
        let ckbtc_distributed: CKBTC = result.iter().map(|e| e.ckbtc_share_amount).sum();
        assert_eq!(tal_distributed.to_u64(), target_borrowed_tal);
        assert_eq!(ckbtc_distributed.to_u64(), ckbtc_margin);
    }

    #[test]
    fn proptest_distribute_across_lps(
        provided_liquidity_map in btree_map(arb_principal(), arb_usd_amount(), 1..10),
        vault_borrowed_tal in arb_amount(),
        vault_ckbtc_margin in arb_btc_amount(),
    ) {
        let total_provided_liquidity: TAL = provided_liquidity_map.values().cloned().sum();
        let borrowed_tal = vault_borrowed_tal.min(total_provided_liquidity.to_u64());

        // Ensure that the sum of provided liquidity is greater or equal to vault's borrowed_tal
        prop_assert!(total_provided_liquidity.to_u64() >= borrowed_tal);

        // Call the function
        let result = crate::state::distribute_across_lps(&provided_liquidity_map, TAL::from(borrowed_tal), CKBTC::from(vault_ckbtc_margin));

        // Calculate the sum of distributed values
        let tal_debited: TAL = result.iter().map(|e| e.tal_to_debit).sum();
        let ckbtc_distributed: CKBTC = result.iter().map(|e| e.ckbtc_reward).sum();

        // Assert that the sum of distributed values matches the vault's values
        assert_eq!(tal_debited.to_u64(), borrowed_tal);
        assert_eq!(ckbtc_distributed.to_u64(), vault_ckbtc_margin);
    }
}
