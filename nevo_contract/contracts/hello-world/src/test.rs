#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

// ─── Existing pool tests (unchanged) ─────────────────────────────────────────

#[test]
fn test_create_pool() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let title = String::from_str(&env, "Emergency Relief Fund");
    let description = String::from_str(&env, "Helping those in need");
    let goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);

    assert_eq!(pool_id, 1);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.0, 1);
    assert_eq!(pool.1, creator);
    assert_eq!(pool.2, goal);
    assert_eq!(pool.3, 0);
    assert_eq!(pool.4, false);
}

#[test]
fn test_donate() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let title = String::from_str(&env, "Educational Scholarship");
    let description = String::from_str(&env, "Support for students");
    let goal: u128 = 10_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);

    let donation_amount: u128 = 100_000_000;
    client.donate(&pool_id, &donor, &donation_amount);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.3, donation_amount);
}

#[test]
fn test_multiple_donations() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let donor1 = Address::generate(&env);
    let donor2 = Address::generate(&env);
    let title = String::from_str(&env, "Community Project");
    let description = String::from_str(&env, "Building together");
    let goal: u128 = 5_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);

    client.donate(&pool_id, &donor1, &100_000_000);
    client.donate(&pool_id, &donor2, &200_000_000);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.3, 300_000_000);
}

#[test]
fn test_close_pool() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let title = String::from_str(&env, "Closed Pool");
    let description = String::from_str(&env, "Test pool");
    let goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);
    client.close_pool(&pool_id);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.4, true);
}

#[test]
#[should_panic(expected = "Pool is closed")]
fn test_donate_to_closed_pool() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let title = String::from_str(&env, "Test Pool");
    let description = String::from_str(&env, "Test");
    let goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);
    client.close_pool(&pool_id);

    client.donate(&pool_id, &donor, &100_000_000);
}

#[test]
fn test_multiple_pools() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator1 = Address::generate(&env);
    let creator2 = Address::generate(&env);

    let pool_id_1 = client.create_pool(
        &creator1,
        &String::from_str(&env, "Pool 1"),
        &String::from_str(&env, "First pool"),
        &1_000_000_000,
    );

    let pool_id_2 = client.create_pool(
        &creator2,
        &String::from_str(&env, "Pool 2"),
        &String::from_str(&env, "Second pool"),
        &2_000_000_000,
    );

    assert_eq!(pool_id_1, 1);
    assert_eq!(pool_id_2, 2);
    assert_eq!(client.get_pool_count(), 2);
}

// ─── Milestone tests ──────────────────────────────────────────────────────────

/// Helper: create a pool and return (pool_id, creator).
fn setup_pool(env: &Env, client: &ContractClient, goal: u128) -> (u32, Address) {
    let creator = Address::generate(env);
    let pool_id = client.create_pool(
        &creator,
        &String::from_str(env, "Scholarship Pool"),
        &String::from_str(env, "Graduation journey funding"),
        &goal,
    );
    (pool_id, creator)
}

/// Helper: build a soroban Vec<Milestone> from a slice of (amount, unlock_time).
fn make_milestones(env: &Env, entries: &[(u128, u64)]) -> Vec<Milestone> {
    let mut v: Vec<Milestone> = Vec::new(env);
    for &(amount, unlock_time) in entries {
        v.push_back(Milestone {
            amount,
            unlock_time,
        });
    }
    v
}

// ── Happy path ────────────────────────────────────────────────────────────────

#[test]
fn test_setup_milestones_happy_path() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = 3_000_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    // Three equal milestones that sum to the goal.
    let milestones = make_milestones(
        &env,
        &[
            (1_000_000_000, 1_000),
            (1_000_000_000, 2_000),
            (1_000_000_000, 3_000),
        ],
    );

    client.setup_application_milestones(&pool_id, &student, &milestones);

    // Verify stored milestones are retrievable and correct.
    let stored = client.get_milestones(&pool_id, &student);
    assert_eq!(stored.len(), 3);

    let m0 = stored.get(0).unwrap();
    assert_eq!(m0.amount, 1_000_000_000);
    assert_eq!(m0.unlock_time, 1_000);

    let m2 = stored.get(2).unwrap();
    assert_eq!(m2.amount, 1_000_000_000);
    assert_eq!(m2.unlock_time, 3_000);
}

#[test]
fn test_setup_milestones_single_entry() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = 500_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    // A single milestone covering the full goal is valid.
    let milestones = make_milestones(&env, &[(500_000_000, 9_999)]);

    client.setup_application_milestones(&pool_id, &student, &milestones);

    let stored = client.get_milestones(&pool_id, &student);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored.get(0).unwrap().amount, 500_000_000);
}

#[test]
fn test_setup_milestones_different_students_same_pool() {
    // Two different students can each have their own milestone schedule on the
    // same pool without interfering with each other.
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = 2_000_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);

    let student_a = Address::generate(&env);
    let student_b = Address::generate(&env);

    let milestones_a = make_milestones(&env, &[(1_000_000_000, 100), (1_000_000_000, 200)]);
    let milestones_b = make_milestones(&env, &[(500_000_000, 50), (1_500_000_000, 150)]);

    client.setup_application_milestones(&pool_id, &student_a, &milestones_a);
    client.setup_application_milestones(&pool_id, &student_b, &milestones_b);

    assert_eq!(client.get_milestones(&pool_id, &student_a).len(), 2);
    assert_eq!(client.get_milestones(&pool_id, &student_b).len(), 2);
}

// ── Revert / panic cases ──────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Pool not found")]
fn test_setup_milestones_pool_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let student = Address::generate(&env);
    let milestones = make_milestones(&env, &[(1_000_000_000, 1_000)]);

    // Pool 99 does not exist.
    client.setup_application_milestones(&99u32, &student, &milestones);
}

#[test]
#[should_panic(expected = "Pool is closed")]
fn test_setup_milestones_closed_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = 1_000_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    client.close_pool(&pool_id);

    let student = Address::generate(&env);
    let milestones = make_milestones(&env, &[(1_000_000_000, 1_000)]);

    client.setup_application_milestones(&pool_id, &student, &milestones);
}

#[test]
#[should_panic(expected = "Milestones array must not be empty")]
fn test_setup_milestones_empty_array() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = 1_000_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    let empty: Vec<Milestone> = Vec::new(&env);
    client.setup_application_milestones(&pool_id, &student, &empty);
}

#[test]
#[should_panic(expected = "Sum of milestone amounts must equal the pool goal")]
fn test_setup_milestones_sum_less_than_goal() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = 3_000_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    // Sum = 2_000_000_000 ≠ 3_000_000_000
    let milestones = make_milestones(&env, &[(1_000_000_000, 100), (1_000_000_000, 200)]);
    client.setup_application_milestones(&pool_id, &student, &milestones);
}

#[test]
#[should_panic(expected = "Sum of milestone amounts must equal the pool goal")]
fn test_setup_milestones_sum_greater_than_goal() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = 1_000_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    // Sum = 2_000_000_000 > 1_000_000_000
    let milestones = make_milestones(&env, &[(1_000_000_000, 100), (1_000_000_000, 200)]);
    client.setup_application_milestones(&pool_id, &student, &milestones);
}

#[test]
#[should_panic(expected = "Milestones already set for this student")]
fn test_setup_milestones_no_overwrite_active_locked_funds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = 2_000_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    let milestones = make_milestones(&env, &[(1_000_000_000, 100), (1_000_000_000, 200)]);

    // First call succeeds.
    client.setup_application_milestones(&pool_id, &student, &milestones.clone());

    // Second call must revert — cannot overwrite active locked funds.
    client.setup_application_milestones(&pool_id, &student, &milestones);
}
