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
fn test_increment_guarded_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(IncrementContract, {});
    let client = IncrementContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    // let other = Address::generate(&env);

    // Configure who is permitted
    client.set_owner(&user);

    // Success case: permitted user
    assert_eq!(client.increment_guarded(&user, &5), 5);

    // Uncomment to see error
    // assert_eq!(client.increment_guarded(&other, &5), 5);


    // @todo This panic was causing an issue. Figure out how to gracefully handle a panic in case of a failure
    // So that the negative cases can be tested.
    
    // // Negative case: different user should fail
    // let res = std::panic::catch_unwind(|| {
    //     client.increment_guarded(&other, &1);
    // });
    // assert!(res.is_err(), "expected unauthorized call to panic");
}
