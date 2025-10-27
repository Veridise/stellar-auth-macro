#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
use access_control_macros::{access_control, no_access_control, authorized_by};

#[contracttype]
pub enum DataKey {
    Counter(Address),
    Owner, // <— add this
}

#[contract]
pub struct IncrementContract;

#[contractimpl]
#[access_control]
impl IncrementContract {
    #[no_access_control]
    pub fn increment(env: Env, user: Address, value: u32) -> u32 {
        user.require_auth();
        let key = DataKey::Counter(user.clone());
        let mut count: u32 = env.storage().persistent().get(&key).unwrap_or_default();
        count += value;
        env.storage().persistent().set(&key, &count);
        count
    }

    // Helper to set the expected owner (used by tests)
    #[no_access_control]
    pub fn set_owner(env: Env, owner: Address) {
        env.storage().persistent().set(&DataKey::Owner, &owner);
    }

    /// Uses the macro guard: Self::is_permitted(&env, &user) + user.require_auth()
    #[authorized_by(user, is_permitted)]
    pub fn increment_guarded(env: Env, user: Address, value: u32) -> u32 {
        let key = DataKey::Counter(user.clone());
        let mut count: u32 = env.storage().persistent().get(&key).unwrap_or_default();
        count += value;
        env.storage().persistent().set(&key, &count);
        count
    }

    // Predicate used by #[authorized_by(...)]
    fn is_permitted(env: &Env, user: &Address) -> bool {
        let stored: Option<Address> = env.storage().persistent().get(&DataKey::Owner);
        matches!(stored, Some(ref owner) if owner == user)
    }
}

mod test;
