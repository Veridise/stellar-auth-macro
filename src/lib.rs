//! This contract demonstrates how to implement authorization using
//! Soroban-managed auth framework for a simple case (a single user that needs
//! to authorize a single contract invocation).
//!
//! See `timelock` and `single_offer` examples for demonstration of performing
//! authorized token operations on behalf of the user.
//!
//! See `atomic_swap` and `atomic_multiswap` examples for demonstration of
//! multi-party authorizaton.
//!
//! See `account` example for demonstration of an acount contract with
//! a custom authentication scheme and a custom authorization policy.
#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

// ⬇️ bring in the macros
use access_control_macros::{access_control, no_access_control};

#[contracttype]
pub enum DataKey {
    Counter(Address),
}

#[contract]
pub struct IncrementContract;

#[contractimpl]
#[access_control] // ⬅️ Enforce that all pub fns below must have #[no_access_control]
impl IncrementContract {
    /// Increment increments a counter for the user, and returns the value.
    #[no_access_control] // ⬅️ Explicitly allow this exported method
    pub fn increment(env: Env, user: Address, value: u32) -> u32 {
        // Requires `user` to have authorized call of the `increment` of this
        // contract with all the arguments passed to `increment`, i.e. `user`
        // and `value`. This will panic if auth fails for any reason.
        // When this is called, Soroban host performs the necessary
        // authentication, manages replay prevention and enforces the user's
        // authorization policies.
        // The contracts normally shouldn't worry about these details and just
        // write code in generic fashion using `Address` and `require_auth` (or
        // `require_auth_for_args`).
        user.require_auth();

        // This call is equilvalent to the above:
        // user.require_auth_for_args((&user, value).into_val(&env));

        // The following has less arguments but is equivalent in authorization
        // scope to the above calls (the user address doesn't have to be
        // included in args as it's guaranteed to be authenticated).
        // user.require_auth_for_args((value,).into_val(&env));

        // Construct a key for the data being stored. Use an enum to set the
        // contract up well for adding other types of data to be stored.
        let key = DataKey::Counter(user.clone());

        // Get the current count for the invoker.
        let mut count: u32 = env.storage().persistent().get(&key).unwrap_or_default();

        // Increment the count.
        count += value;

        // Save the count.
        env.storage().persistent().set(&key, &count);

        // Return the count to the caller.
        count
    }

    // --- Uncomment to see the macro error -----------------------------------
    // Missing #[no_access_control] on a public fn will FAIL compilation:
    #[no_access_control] // ⬅️ Explicitly allow this exported method
    pub fn missing_marker(_env: Env) {
        // ...
    }
    // ------------------------------------------------------------------------
}

mod test;
