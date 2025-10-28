#![cfg(test)]
extern crate std;

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, Env, IntoVal,
};

use crate::{IncrementContract, IncrementContractClient};

#[test]
fn test_increment_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(IncrementContract, {});
    let client = IncrementContractClient::new(&env, &contract_id);

    let user_1 = Address::generate(&env);
    let user_2 = Address::generate(&env);

    assert_eq!(client.increment(&user_1, &5), 5);

    // Verify that the user indeed had to authorize a call of `increment`
    assert_eq!(
        env.auths(),
        std::vec![(
            user_1.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    contract_id.clone(),
                    symbol_short!("increment"),
                    (user_1.clone(), 5_u32).into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    // Additional calls; no further auth assertions required here
    assert_eq!(client.increment(&user_1, &2), 7);
    assert_eq!(client.increment(&user_2, &1), 1);
    assert_eq!(client.increment(&user_1, &3), 10);
    assert_eq!(client.increment(&user_2, &4), 5);
}

#[test]
fn test_increment_owner_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(IncrementContract, {});
    let client = IncrementContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    // let other = Address::generate(&env);

    // Configure who is permitted
    client.set_owner(&user);

    // Success case: allowed user - admin
    assert_eq!(client.increment_owner(&user, &5), 5);
}

#[test]
#[should_panic(expected = "unauthorized")]
fn test_increment_owner_auth_denied_should_panic() {
    use soroban_sdk::{Address, Env};

    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(crate::IncrementContract, {});
    let client = crate::IncrementContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let other = Address::generate(&env);

    client.set_owner(&owner); // only `owner` is permitted

    // This should panic due to the caller not fulfilling auth requirements
    client.increment_owner(&other, &1);
}
