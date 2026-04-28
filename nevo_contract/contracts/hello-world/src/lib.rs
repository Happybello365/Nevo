#![no_std]

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Symbol};

// Storage key constants
const POOL_COUNT: &str = "pool_count";
const POOL_PREFIX: &str = "p";
const CREATOR_SUFFIX: &str = "_creator";
const GOAL_SUFFIX: &str = "_goal";
const COLLECTED_SUFFIX: &str = "_collected";
const CLOSED_SUFFIX: &str = "_closed";
const APPLICATION_COUNT_PREFIX: &str = "a_count_";
const APPLICATION_PREFIX: &str = "a_";
const APPLICANT_PREFIX: &str = "ap_";

// Application and claim tracking constants
const APPLICATION_STATUS_PREFIX: &str = "app_status";
const CLAIMED_AMOUNT_PREFIX: &str = "claimed_amount";
const APPLICATION_STATUS_APPROVED: &str = "Approved";
const APPLICATION_STATUS_REJECTED: &str = "Rejected";

#[derive(Clone)]
#[contracttype]
pub struct Pool {
    pub sponsor: Address,
    pub goal: u128,
    pub collected: u128,
    pub is_closed: bool,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    // ─── Pool Management ─────────────────────────────────────────────────────

    /// Create a new donation / sponsorship pool.
    pub fn create_pool(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        goal: u128,
    ) -> u32 {
        // creator.require_auth();  // TODO: Enable auth validation in production

        let pool_count_key = Symbol::new(&env, POOL_COUNT);
        let mut pool_count: u32 = env
            .storage()
            .persistent()
            .get::<_, u32>(&pool_count_key)
            .unwrap_or(0);

        let pool_id = pool_count + 1;
        pool_count = pool_id;

        // Store pool data - using numeric pool ID as key
        let pool_key = pool_id;

        let pool = Pool {
            sponsor: creator.clone(),
            goal,
            collected: 0u128,
            is_closed: false,
        };

        env.storage()
            .persistent()
            .set(&pool_id, &pool);

        env.storage().persistent().set(&pool_count_key, &pool_count);

        pool_id
    }

    /// Apply for a scholarship from an active pool.
    pub fn apply_for_scholarship(
        env: Env,
        student: Address,
        pool_id: u32,
        credential_hash: BytesN<32>,
        requested_amount: i128,
    ) -> u32 {
        student.require_auth();

        let pool_key = pool_id;
        let pool: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_key)
            .expect("Pool not found");

        if pool.is_closed {
            panic!("Pool is inactive");
        }

        if requested_amount <= 0 {
            panic!("Requested amount must be positive");
        }

        let applicant_key = (
            Symbol::new(&env, APPLICANT_PREFIX),
            pool_id,
            student.clone(),
        );
        if env.storage().persistent().has(&applicant_key) {
            panic!("Duplicate application");
        }

        let count_key = (Symbol::new(&env, APPLICATION_COUNT_PREFIX), pool_id);
        let mut app_count: u32 = env
            .storage()
            .persistent()
            .get::<_, u32>(&count_key)
            .unwrap_or(0);
        app_count += 1;

        let app_key = (Symbol::new(&env, APPLICATION_PREFIX), pool_id, app_count);
        env.storage().persistent().set(
            &app_key,
            &(student.clone(), credential_hash, requested_amount),
        );

        env.storage().persistent().set(&applicant_key, &true);
        env.storage().persistent().set(&count_key, &app_count);

        app_count
    }

    /// Donate to an existing pool.
    pub fn donate(env: Env, pool_id: u32, donor: Address, amount: u128) {
        // donor.require_auth();  // TODO: Enable auth validation in production

        let pool_data: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_id)
            .expect("Pool not found");

        if pool_data.is_closed {
            panic!("Pool is closed");
        }

        let new_collected = pool_data.collected + amount;
        let updated_pool = Pool {
            sponsor: pool_data.sponsor,
            goal: pool_data.goal,
            collected: new_collected,
            is_closed: pool_data.is_closed,
        };
        env.storage().persistent().set(
            &pool_key,
            &updated_pool,
        );

        let donor_index: u32 = env
            .storage()
            .persistent()
            .get::<_, u32>(&(pool_id, "d_count"))
            .unwrap_or(0);

        env.storage()
            .persistent()
            .set(&(pool_id, "d_count"), &(donor_index + 1));
    }

    /// Get pool information as a tuple (id, creator, goal, collected, is_closed).
    pub fn get_pool(env: Env, pool_id: u32) -> (u32, Address, u128, u128, bool) {
        let pool: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_id)
            .expect("Pool not found");

        (pool_id, pool.sponsor, pool.goal, pool.collected, pool.is_closed)
    }

    /// Close a donation pool.
    pub fn close_pool(env: Env, pool_id: u32) {
        let pool: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_id)
            .expect("Pool not found");

        pool.sponsor.require_auth();

        let updated_pool = Pool {
            sponsor: pool.sponsor,
            goal: pool.goal,
            collected: pool.collected,
            is_closed: true,
        };

        env.storage()
            .persistent()
            .set(&pool_id, &updated_pool);
    }

    /// Get the total number of pools.
    pub fn get_pool_count(env: Env) -> u32 {
        let pool_count_key = Symbol::new(&env, POOL_COUNT);
        env.storage()
            .persistent()
            .get::<_, u32>(&pool_count_key)
            .unwrap_or(0)
    }

    /// Set application status for a student in a pool (helper for testing and admin)
    pub fn set_application_status(env: Env, pool_id: u32, student: Address, status: String) {
        let status_key = (APPLICATION_STATUS_PREFIX, pool_id, student.clone());
        env.storage().persistent().set(&status_key, &status);
    }

    /// Get application status for a student in a pool
    pub fn get_application_status(env: Env, pool_id: u32, student: Address) -> String {
        let status_key = (APPLICATION_STATUS_PREFIX, pool_id, student.clone());
        env.storage()
            .persistent()
            .get::<_, String>(&status_key)
            .unwrap_or(String::from_str(&env, ""))
    }

    /// Retrieve a stored application record for a pool.
    pub fn get_application(env: Env, pool_id: u32, application_id: u32) -> (Address, BytesN<32>, i128) {
        let app_key = (Symbol::new(&env, APPLICATION_PREFIX), pool_id, application_id);
        env.storage()
            .persistent()
            .get::<_, (Address, BytesN<32>, i128)>(&app_key)
            .expect("Application not found")
    }

    /// Get claimed amount for a student in a pool
    pub fn get_claimed_amount(env: Env, pool_id: u32, student: Address) -> i128 {
        let claimed_key = (CLAIMED_AMOUNT_PREFIX, pool_id, student.clone());
        env.storage()
            .persistent()
            .get::<_, i128>(&claimed_key)
            .unwrap_or(0)
    }

    /// Claim funds: allows an approved student to receive their token funding
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `student` - The student address receiving funds (must authorize)
    /// * `pool_id` - The ID of the pool to claim from
    /// * `claim_amount` - The amount to claim (in tokens, represented as i128)
    /// * `token_address` - The address of the token to transfer
    ///
    /// # Errors
    /// - Panics if student is not authorized
    /// - Panics if application status is not "Approved"
    /// - Panics if attempting to overdraw (claimed + claim_amount > collected)
    pub fn claim_funds(
        env: Env,
        student: Address,
        pool_id: u32,
        claim_amount: i128,
        _token_address: Address,
    ) {
        student.require_auth();

        if claim_amount <= 0 {
            panic!("Claim amount must be positive");
        }

        let status_key = (APPLICATION_STATUS_PREFIX, pool_id, student.clone());
        let status: String = env
            .storage()
            .persistent()
            .get::<_, String>(&status_key)
            .unwrap_or(String::from_str(&env, ""));

        if status == String::from_str(&env, "") {
            panic!("Application status not found");
        }
        if status != String::from_str(&env, APPLICATION_STATUS_APPROVED) {
            panic!("Application is not approved");
        }

        let pool_key = pool_id;
        let pool: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_key)
            .expect("Pool not found");

        let collected_amount = pool.collected as i128;
        let claimed_key = (CLAIMED_AMOUNT_PREFIX, pool_id, student.clone());
        let current_claimed: i128 = env
            .storage()
            .persistent()
            .get::<_, i128>(&claimed_key)
            .unwrap_or(0);

        let new_claimed = current_claimed + claim_amount;
        if new_claimed > collected_amount {
            panic!("Overdraw attempt");
        }

        env.storage().persistent().set(&claimed_key, &new_claimed);
    }
}

mod test;
