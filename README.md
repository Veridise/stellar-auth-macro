## Overview

This crate  provides a  **simple, explicit  access-control mechanism**
for  Stellar/Soroban contracts,  removing the  need for  ad-hoc checks
scattered throughout  the codebase.  It  was developed and  audited by
Veridise security experts.

## Before applying the crate

Access control is implicit and easy to overlook.

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

* With this order, `access_control` instruments your methods **first**, strips `#[authorized_by(..)]`, and hands a clean, already-guarded `impl` to `#[contractimpl]`.
* If you reverse the order, `contractimpl` might synthesize wrappers and the original `#[authorized_by(..)]` could land on a non-function item. To avoid noisy analyzer errors, the standalone `#[authorized_by]` attribute in this crate is tolerant: if it doesn’t see a function/method shape (or required params), it simply leaves the item unchanged and warns at most. Still, the **recommended** order is `#[access_control]` then `#[contractimpl]`.

### Predicates: what they must look like

Write your predicate to be **deterministic** and **read-only**: no storage writes, no auth calls, and no non-determinism. The macro expects the signature:

```rust
fn predicate(env: &Env, who: &Address) -> bool
```

It can be:

* an inherent method on the same type (e.g. `fn only_owner(&Env, &Address) -> bool`), referenced as `only_owner` (the macro rewrites to `Self::only_owner`), or
* any path like `crate::auth::is_admin`.

Calling `require_auth()` inside the predicate is *not* necessary; the macro injects that **after** the predicate check passes. Typically, predicates should answer only “is this address allowed, given the current on-chain state?”

### External modules are ignored (by design)

`#[access_control]` works on:

* an `impl` block, or
* an **inline** `mod` (the content is present in the same file).

If you put it on an **external** module (declared with `mod x;` and defined elsewhere), the macro **cannot** inspect the contents. In that case it raises a hard error. Therefore to use the access control macro, use it directly on the `impl` (or inline the module).

### Macro expansion: Before and After

Before:

```rust
impl MyContract {
    // open entrypoint
    #[no_access_control]
    pub fn balance_of(env: Env, user: Address) -> u128 {
        // ...
    }

    #[authorized_by(caller, only_owner)]
    pub fn change_owner(env: Env, caller: Address, new_owner: Address) {
        // ...
    }
}
```

After `#[access_control]`:

```rust
impl MyContract {
    // Predicate used by authorized_by()
    fn only_owner(env: &Env, who: &Address) -> bool {
        env.storage().persistent().get::<_, Address>(&DataKey::Owner)
            .map_or(false, |owner| &owner == who)
    }

    fn is_owner(env: &Env, user: &Address) -> bool { /* unchanged */ }
}
```

## How it works
The macro injects both of the following for every function marked as authorized:
- the authorization predicate check
- the required `require_auth()` call

The build fails at compile time if any public function in the `impl` block is missing either:
- an authorization `#[authorized_by(user, predicate)]` annotation, or
- an explicit `#[no_access_control]` marker

## Why this matters
- Developers are protected from accidentally introducing unguarded privileged functions.
- Access-control mistakes surface at compile time, rather than silently at runtime.
- Security auditors can reason about the authorization model quickly, with
  unconstrained privileged functions standing out immediately via `#[no_access_control]`.

*In short, this crate turns access control from an implicit convention into an
explicit, enforceable contract.*

# Usage

Add the macros  crate to your project, then import  the attributes. In
`Cargo.toml`, point to the repository and a pinned revision.

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

Annotate your contract  implementation with `#[access_control]` placed
**above**  `#[contractimpl]`. This  order  ensures the  guard code  is
injected  before  Soroban  generates  client stubs.  For  each  public
entrypoint,  mark it  as  open with  `#[no_access_control]` (no  guard
injected), or  protected with `#[authorized_by(arg,  predicate)]` (the
macro injects a predicate check  and `require_auth()` on the specified
argument). See the example above.
