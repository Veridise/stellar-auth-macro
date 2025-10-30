#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
use access_control_macros::{access_control, no_access_control, authorized_by};

#[contracttype]
pub enum DataKey {
    Counter(Address),
    Owner, // owner of the program
}

#[contract]
pub struct IncrementContract;

impl IncrementContract {
    // Predicate used by #[authorized_by(...)]
    fn only_owner(env: &Env, user: &Address) -> bool {
        let stored: Option<Address> = env.storage().persistent().get(&DataKey::Owner);
        matches!(stored, Some(ref owner) if owner == user)
    }
}

#[access_control]
#[contractimpl]
impl IncrementContract {
        // 1) Set owner during deployment/init (one-time).
    #[no_access_control]
    pub fn initialize(env: Env, owner: Address) {
        if env.storage().persistent().has(&DataKey::Owner) {
            panic!("already initialized");
        }
        // Ensure the declared owner actually authorized this init call.
        owner.require_auth();

        env.storage().persistent().set(&DataKey::Owner, &owner);
    }

    // 2) Example of a protected method that only the owner can call.
    //    Your #[authorized_by] macro will inject: only_owner(&env, &caller) && caller.require_auth()
    #[authorized_by(caller, only_owner)]
    pub fn change_owner(env: Env, caller: Address, new_owner: Address) {
        env.storage().persistent().set(&DataKey::Owner, &new_owner);
    }

    #[no_access_control]
    pub fn increment(env: Env, user: Address, value: u32) -> u32 {
        user.require_auth();
        let key = DataKey::Counter(user.clone());
        let mut count: u32 = env.storage().persistent().get(&key).unwrap_or_default();
        count += value;
        env.storage().persistent().set(&key, &count);
        count
    }

    /// Uses the macro guard: Self::only_owner(&env, &user) + user.require_auth()
    #[authorized_by(user, only_owner)]
    pub fn increment_owner(env: Env, user: Address, value: u32) -> u32 {
        let key = DataKey::Counter(user.clone());
        let mut count: u32 = env.storage().persistent().get(&key).unwrap_or_default();
        count += value;
        env.storage().persistent().set(&key, &count);
        count
    }
}

mod test;
