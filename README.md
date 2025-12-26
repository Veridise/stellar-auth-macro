# Advisory

This project is experimental and unaudited. Use at your own risk and review generated code and unit tests before deploying to mainnet.

# Overview

This crate  provides a  **simple, explicit  access-control mechanism**
for  Stellar/Soroban contracts,  removing the  need for  ad-hoc checks
scattered throughout  the codebase.  It  was developed and  audited by
Veridise security experts.

## Before applying the crate

In this version, access control is implicit and easy to overlook.

```rust
#[contractimpl]
impl MyContract {
    pub fn balance_of(env: Env, user: Address) -> u128 {
        // ...
    }

    pub fn change_owner(env: Env, caller: Address, new_owner: Address) {
        // ...
    }
}
```

## After applying the crate

After applying this crate, access control becomes explicit and self-documenting.
Each public function clearly states whether it is unrestricted or protected, and
under what conditions.

```rust
#[access_control]
#[contractimpl]
impl MyContract {

    #[no_access_control]
    pub fn balance_of(env: Env, user: Address) -> u128 {
        // ...
    }

    #[authorized_by(caller, only_owner)]
    pub fn change_owner(env: Env, caller: Address, new_owner: Address) {
        // `only_owner(caller)` check has to pass before this function executes
        // ...
    }
}

impl MyContract {
    // Predicate used by authorized_by()
    fn only_owner(env: &Env, who: &Address) -> bool {
        env.storage()
            .persistent()
            .get::<_, Address>(&DataKey::Owner)
            .map_or(false, |owner| &owner == who)
    }
}
```

## Why this matters

- Developers are protected from accidentally introducing unguarded privileged functions.
- Access-control mistakes surface at compile time, rather than silently at runtime.
- Security auditors can reason about the authorization model quickly, with
  unconstrained privileged functions standing out immediately via `#[no_access_control]`.

*In short, this crate turns access control from an implicit convention into an
explicit, enforceable contract.*

## How it works

The macro injects both of the following for every function marked as authorized:

- the authorization predicate check
- the required `require_auth()` call

The build fails at compile time if any public function in the `impl` block is missing either:

- an authorization `#[authorized_by(user, predicate)]` annotation, or
- an explicit `#[no_access_control]` marker

## Usage

Add the macros  crate to your project, then import  the attributes. In
`Cargo.toml`, point to the repository and a pinned revision.

```toml
# Cargo.toml
[dependencies]
soroban-sdk = { version = "23.0.1" }
access_control_macros = { git = "https://github.com/Veridise/stellar-auth-macro.git", rev = "abcdef1" }
```

In your code, `use` the three attributes as follows:

```rust
use access_control_macros::{access_control, no_access_control, authorized_by};
```

Annotate your contract  implementation with `#[access_control]` placed
above  `#[contractimpl]`. This  order  ensures the  guard code  is
injected  before  Soroban  generates  client stubs.

For    each    public   entrypoint,    mark    it    as   open    with
`#[no_access_control]` - no  guard   injected,  or   protected  with
`#[authorized_by(arg,  predicate)]` - the  macro injects  a  predicate
check and `require_auth()` on the specified argument. See the example
above.

## Disclaimer

This software  is provided "as is"  and without any warranties  of any
kind, whether express or implied. While reasonable care has been taken
in  its design  and development,  no guarantee  is made  regarding its
security,  correctness, or  fitness  for any  particular purpose.   By
using this code, you acknowledge that you  do so at your own risk, and
that the  authors, contributors,  and maintainers assume  no liability
for any damages, losses, or security incidents that may arise from its
use, misuse,  or inability  to function as  intended.  It  is strongly
recommended  to conduct  independent  security  reviews, testing,  and
validation before deploying this software in production environments.
