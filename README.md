# Advisory

This project is experimental and unaudited. Use at your own risk and review generated code and unit tests before deploying to mainnet.

# Overview

When writing smart contracts, it is easy to get access control wrong.
In the Soroban ecosystem, developers used to Solidity make several common errors.
This is because there is no global `msg.sender`. Instead, an `Address` is used as an argument for authorization logic. Any time a privileged action is taken one must both verify the address is relevant (for example that it is the `owner`) and prove the caller actually authorized the invocation with `require_auth()`.
Forgetting one or both of these two steps is easy to do as the number of functions and/or contracts within a project grow.
This crate asks: why not let the compiler remember for you?

This crate adds a tiny, declarative access-control layer so you can state intent and let a proc-macro enforce it. Put `#[access_control]` on your contract `impl` block. Once done, the compiler will require you to mark each public entrypoint as either

* explicitly open with `#[no_access_control]`
* protected with `#[authorized_by(arg, predicate)]`.

The macro injects both the predicate check and the `require_auth()` call for any function tagged for authorization, and it fails to build if any public function is missing either one of these annotations.

This is useful for both developers and auditors! Developers can rest easy knowing that forgetting access control
on newly added or updated functions will trigger an error, instead of silently succeeding.
Auditors can understand the protocol more quickly, spotting a glaring red flag whenever a privileged
function is explicitly marked with `#[no_access_control]`.

# Usage

Add the macros crate to your project directly from GitHub, then import the attributes. In `Cargo.toml`, point to the repository and a pinned revision.

```toml
# Cargo.toml
[dependencies]****
soroban-sdk = { version = "23.0.1" }
access_control_macros = { git = "https://github.com/Veridise/stellar-auth-macro.git", rev = "abcdef1" }
```

In your code, `use` the three attributes as follows:

```rust
use access_control_macros::{access_control, no_access_control, authorized_by};
```

Annotate your contract implementation with `#[access_control]` placed **above** `#[contractimpl]`. This order ensures the guard code is injected before Soroban generates client stubs. For each public entrypoint, mark it as open with `#[no_access_control]` (no guard injected), or protected with `#[authorized_by(arg, predicate)]` (the macro injects a predicate check and `require_auth()` on the specified argument). A minimal example looks like this:

```rust
#[access_control]
#[contractimpl]
impl MyContract {
    // Open endpoint — no guard injected.
    #[no_access_control]
    pub fn balance_of(env: Env, user: Address) -> u128 {
        // ...
    }

    // Protected endpoint — macro injects at the top:
    //   if !Self::only_owner(&env, &caller) { panic!("unauthorized: ...") }
    //   caller.require_auth();
    #[authorized_by(caller, only_owner)]
    pub fn change_owner(env: Env, caller: Address, new_owner: Address) {
        env.storage().persistent().set(&DataKey::Owner, &new_owner);
    }
}

impl MyContract {
    // Predicate used by authorized_by()
    fn only_owner(env: &Env, who: &Address) -> bool {
        env.storage().persistent().get::<_, Address>(&DataKey::Owner)
            .map_or(false, |owner| &owner == who)
    }
}
```

# Developers

## Building

To build the contracts you need a couple prerequisites:

* A recent stable [Rust](https://www.rust-lang.org/) toolchain
* An editor that supports Rust
* [Stellar CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup#install-the-stellar-cli)

See the [documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) for more prerequisite installation instructions.

The stellar contracts can then be built with ```make build```.

## Testing

The repository also contains some tests which demonstrate the macros in action. These can be run with ```make test```.
