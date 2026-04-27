#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol, Vec};

// Storage key constants
const POOL_COUNT: &str = "pool_count";

// ─── Data Types ──────────────────────────────────────────────────────────────

/// A single time-based funding milestone for a student's graduation journey.
#[contracttype]
#[derive(Clone)]
pub struct Milestone {
    /// Amount (in stroops) to be released at this milestone.
    pub amount: u128,
    /// Ledger timestamp (Unix seconds) after which this milestone unlocks.
    pub unlock_time: u64,
}

/// Composite storage key: milestones for a specific student within a pool.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    PoolCount,
    Pool(u32),
    DonationCount(u32),
    /// Milestones allocated to `student` inside `pool_id`.
    Milestones(u32, Address),
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

        env.storage()
            .persistent()
            .set(&pool_id, &(creator.clone(), goal, 0u128, false));

        env.storage().persistent().set(&pool_count_key, &pool_count);

        pool_id
    }

    /// Donate to an existing pool.
    pub fn donate(env: Env, pool_id: u32, donor: Address, amount: u128) {
        // donor.require_auth();  // TODO: Enable auth validation in production

        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

        if pool_data.3 {
            panic!("Pool is closed");
        }

        let new_collected = pool_data.2 + amount;
        env.storage().persistent().set(
            &pool_id,
            &(pool_data.0.clone(), pool_data.1, new_collected, pool_data.3),
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
        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

        (pool_id, pool_data.0, pool_data.1, pool_data.2, pool_data.3)
    }

    /// Close a donation pool.
    pub fn close_pool(env: Env, pool_id: u32) {
        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

        // pool_data.0.require_auth();  // TODO: Enable auth validation in production

        env.storage()
            .persistent()
            .set(&pool_id, &(pool_data.0, pool_data.1, pool_data.2, true));
    }

    /// Get the total number of pools.
    pub fn get_pool_count(env: Env) -> u32 {
        let pool_count_key = Symbol::new(&env, POOL_COUNT);
        env.storage()
            .persistent()
            .get::<_, u32>(&pool_count_key)
            .unwrap_or(0)
    }

    // ─── Milestone Management ─────────────────────────────────────────────────

    /// Allocate pool funds into time-based milestones for a student's graduation journey.
    ///
    /// # Arguments
    /// * `pool_id`    – The pool whose funds are being scheduled.
    /// * `student`    – The beneficiary address.
    /// * `milestones` – Ordered list of [`Milestone`] entries (amount + unlock_time).
    ///
    /// # Invariants enforced
    /// * The pool must exist and must not be closed.
    /// * The pool creator (validator) must authorise this call.
    /// * `milestones` must be non-empty.
    /// * The sum of all milestone amounts must equal the pool's goal exactly.
    /// * No milestones may already be active for this (pool, student) pair —
    ///   prevents silent overwrite of locked funds.
    pub fn setup_application_milestones(
        env: Env,
        pool_id: u32,
        student: Address,
        milestones: Vec<Milestone>,
    ) {
        // ── 1. Load and validate the pool ────────────────────────────────────
        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

        if pool_data.3 {
            panic!("Pool is closed");
        }

        // ── 2. Authorisation — must match the approval (validator) identity ──
        let validator: Address = pool_data.0.clone();
        validator.require_auth();

        // ── 3. Milestones array must not be empty ────────────────────────────
        if milestones.is_empty() {
            panic!("Milestones array must not be empty");
        }

        // ── 4. Guard: do not overwrite already-active milestones ─────────────
        let milestone_key = DataKey::Milestones(pool_id, student.clone());
        let already_set: bool = env.storage().persistent().has(&milestone_key);

        if already_set {
            panic!("Milestones already set for this student; cannot overwrite active locked funds");
        }

        // ── 5. Mathematical invariant: sum(amounts) == pool goal ─────────────
        let pool_goal: u128 = pool_data.1;
        let mut total: u128 = 0u128;

        for i in 0..milestones.len() {
            let m = milestones.get(i).unwrap();
            total = total
                .checked_add(m.amount)
                .expect("Milestone amount overflow");
        }

        if total != pool_goal {
            panic!("Sum of milestone amounts must equal the pool goal");
        }

        // ── 6. Persist milestones ─────────────────────────────────────────────
        env.storage().persistent().set(&milestone_key, &milestones);
    }

    /// Retrieve the milestones allocated to a student for a given pool.
    pub fn get_milestones(env: Env, pool_id: u32, student: Address) -> Vec<Milestone> {
        let milestone_key = DataKey::Milestones(pool_id, student);
        env.storage()
            .persistent()
            .get::<_, Vec<Milestone>>(&milestone_key)
            .expect("No milestones found for this student")
    }
}

mod test;
