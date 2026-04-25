#![cfg(test)]

use crate::{
    base::types::PoolConfig,
    crowdfunding::{CrowdfundingContract, CrowdfundingContractClient},
};
use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env, String};

fn setup(env: &Env) -> (CrowdfundingContractClient<'_>, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register(CrowdfundingContract, ());
    let client = CrowdfundingContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(Address::generate(env))
        .address();
    client.initialize(&admin, &token_address, &0);
    (client, admin, token_address)
}

fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(to, &amount);
}

fn make_pool(env: &Env, token_address: &Address) -> PoolConfig {
    PoolConfig {
        name: String::from_str(env, "Test Pool"),
        description: String::from_str(env, "desc"),
        target_amount: 1_000i128,
        min_contribution: 0,
        is_private: false,
        duration: 86_400,
        created_at: env.ledger().timestamp(),
        token_address: token_address.clone(),
        validator: Address::generate(env),
    }
}

/// Adding 10 pools results in a tracker with exactly 10 correct IDs.
#[test]
fn test_tracker_contains_10_pools() {
    let env = Env::default();
    let (client, _, token) = setup(&env);

    let mut expected_ids = soroban_sdk::Vec::new(&env);
    for _ in 0..10 {
        let creator = Address::generate(&env);
        let config = make_pool(&env, &token);
        mint(&env, &token, &creator, config.target_amount);
        let pool_id = client.create_pool(&creator, &config);
        expected_ids.push_back(pool_id);
    }

    let active = client.list_active_pools();
    assert_eq!(active.len(), 10);
    for id in expected_ids.iter() {
        assert!(active.contains(id));
    }
}

/// Removing a pool from the middle shrinks the tracker by 1 (swap-remove).
#[test]
fn test_tracker_swap_remove_middle() {
    let env = Env::default();
    let (client, admin, token) = setup(&env);

    // Create 5 pools; IDs will be 1..=5
    let mut ids = soroban_sdk::Vec::new(&env);
    for _ in 0..5 {
        let creator = Address::generate(&env);
        let config = make_pool(&env, &token);
        mint(&env, &token, &creator, config.target_amount);
        ids.push_back(client.create_pool(&creator, &config));
    }
    assert_eq!(client.list_active_pools().len(), 5);

    // Close the middle pool (ID 3) — admin closes after marking it Cancelled first
    use crate::base::types::PoolState;
    client.update_pool_state(&ids.get(2).unwrap(), &admin, &PoolState::Cancelled);
    client.close_pool(&ids.get(2).unwrap(), &admin);

    let active = client.list_active_pools();
    assert_eq!(active.len(), 4);
    assert!(!active.contains(ids.get(2).unwrap()));
}
