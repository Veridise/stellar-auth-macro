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

#[contractimpl]
#[access_control]
impl IncrementContract {
    #[no_access_control]
    pub fn increment(env: Env, user: Address, value: u32) -> u32 {
        let key = DataKey::Counter(user.clone());
        let mut count: u32 = env.storage().persistent().get(&key).unwrap_or_default();
        count += value;
        env.storage().persistent().set(&key, &count);
        count
    }

    // @todo The owner key should be updated during deployment, and then successive updates require checking for owner auth.
    // Helper to set the expected owner (used by tests)
    #[no_access_control]
    pub fn set_owner(env: Env, owner: Address) {
        env.storage().persistent().set(&DataKey::Owner, &owner);
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

    // Predicate used by #[authorized_by(...)]
    fn only_owner(env: &Env, user: &Address) -> bool {
        let stored: Option<Address> = env.storage().persistent().get(&DataKey::Owner);
        matches!(stored, Some(ref owner) if owner == user)
    }
}

mod test;
