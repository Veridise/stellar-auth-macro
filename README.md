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

## Details

### What the macros do (and what code they inject)

When you tag an `impl` with `#[access_control]`, the macro walks every method in that block and inspects its attributes. For each method marked `#[authorized_by(arg, predicate_path)]`, it rewrites the function by injecting a guard at the very start of the body and then strips the `#[authorized_by(..)]` attribute so downstream macros never see it. Everything else is left as-is unless you add other annotations.

Injected guard (conceptually):

```rust
{
    if !(predicate_path(&env, &arg)) {
        ::core::panic!("unauthorized: predicate_path(env,arg) failed");
    }
    arg.require_auth();
    /* original body follows */
}
```

For each public method (public visibility **or** in a trait `impl` **or** inside an `impl` that will be passed to `#[contractimpl]`), the macro **enforces** that the method is either:

* marked `#[no_access_control]` (explicitly open), or
* marked `#[authorized_by(..)]` (protected).

If neither is present, the build fails with a clear error telling you which method needs an annotation.

> Having public visibility means the item is declared `pub`, `pub(crate)`, `pub(super)`, or `pub(in …)`, or it appears in a trait `impl`, or the `impl` carries `#[contractimpl]` (which turns methods into external entrypoints).

### Recommended attribute order (multiple macros)

Put `#[access_control]` **above** `#[contractimpl]`:

```rust
#[access_control]
#[contractimpl]
impl MyContract { /* methods */ }
```

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
    pub fn view_balance(env: Env) { /* ... */ }

    // protected entrypoint
    #[authorized_by(user, only_owner)]
    pub fn increment_balance(env: Env, user: Address, n: u32) {
        /* body */
    }

    // predicate (pure; read-only)
    fn is_owner(env: &Env, user: &Address) -> bool {
        let owner: Option<Address> = env.storage().persistent().get(&DataKey::Owner);
        matches!(owner, Some(ref o) if o == user)
    }
}
```

After `#[access_control]`:

```rust
impl MyContract {
    pub fn view_balance(env: Env) { /* unchanged */ }

    pub fn increment_balance(env: Env, user: Address, n: u32) {
        if !(Self::is_owner(&env, &user)) {
            ::core::panic!("unauthorized: only_owner(env,user) failed");
        }
        user.require_auth();
        /* body */
    }

    fn is_owner(env: &Env, user: &Address) -> bool { /* unchanged */ }
}
```

### Referencing predicates by path

You can point to a predicate outside the `impl` using:

```rust
#[authorized_by(caller, crate::auth::is_admin)]
pub fn transfer_fees(env: Env, caller: Address) { /* ... */ }
```

As long as `crate::auth::is_admin(&Env, &Address) -> bool` exists, the guard will inject cleanly.

### What happens if you forget an annotation?

If a public function in the `impl` has **no** `#[no_access_control]` and **no** `#[authorized_by(..)]`, compilation fails with:

```
public method {<name>} is missing #[no_access_control] or #[authorized_by(...)]
```

Add the appropriate attribute and rebuild.

### Example: end-to-end

```rust
#[access_control]
#[contractimpl]
impl GenericLendingProtocol {
    #[no_access_control]
    pub fn version(_env: Env) -> u32 { 1 }

    #[authorized_by(caller, only_owner)]
    pub fn transfer_fee(env: Env, caller: Address, bps: i64) {
        // Guard is injected here:
        //   if !(Self::only_owner(&env, &caller)) panic!("unauthorized");
        //   caller.require_auth();
        // Then your logic:
        send_fee(&env, caller);
    }

    // #[contractimpl] generates external contract entrypoints for every function inside that impl block,
    // regardless of Rust visibility. Even methods without pub will be exported as callable contract functions.
    // If you want the predicate to be private, put it outside the #[contractimpl] block as a free fn or in a separate impl
    // without the attribute
    fn only_owner(env: &Env, who: &Address) -> bool {
        // Owner should be set within an initializer
        let owner: Option<Address> = env.storage().persistent().get(&DataKey::Owner);
        matches!(owner, Some(ref o) if o == user)
    }
}
```

Place `#[access_control]` **above** `#[contractimpl]`, keep predicates read-only, and mark every public entrypoint as **#[no_access_control]** or **#[authorized_by()]**. The macro handles the rest, injecting checks, ordering safely with `contractimpl`, ignoring external modules, and surfacing compile-time errors when something’s missing.

## Limitations

This macro is purposefully small and opinionated by design. It only instruments functions inside annotated `impl` block and does not operate on code generated elsewhere (for example, client stubs or wrappers emitted by other macros). It scans “public” methods—trait impls, anything in an `impl` also tagged with `#[contractimpl]`, or any method with non-private visibility, and requires each `pub` method to be marked `#[no_access_control]` or `#[authorized_by(...)]`.

In `#[access_control]` `impl` blocks, the procedural macro will cause a compiler error if the access-controlled method signature
does not include an `env` parameter named `env`, if no parameter matches the identifier specified
in `#[authorized_by]`, or if that parameter does not support `require_auth()`.
However, `#[authorized_by]` only functions correctly if inside a `#[access_control]` `impl` block.
If the outer macro is not present, the inner attribute is left in place and the macro quietly skips instrumentation.

For predicate resolution, a single-segment name is invoked as `Self::predicate`, otherwise the path is used as written. The macro calls it before `require_auth()`. The predicate should only be used to perform the intended verification, and should not modify any state. The macro does not **currently** provide role composition, multi-sig policies, or reentrancy protection, and it doesn’t rewrite or verify the logic inside your predicate.

Interacting proc-macros can still confuse IDEs. We bias toward being non-fatal for unresolved shapes to avoid rust-analyzer spam, but you may see stale diagnostics until a clean build. Finally, `#[access_control]` must be placed on an inline `impl` (or inline `mod`). External modules are rejected because their contents aren’t visible at macro time.

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

## Contributing

Open a small PR with a focused change and a matching test. Run formatting locally:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

If you change the macro instrumentation logic, include:

* a positive test showing the injected guard runs
* a negative test proving a missing attribute is caught
* A PR description to illustrate the motivation and transformation

Please keep the macro behavior predictable and the error messages short and actionable.

# TODOS

The following improvements/additions to the macro are in the pipeline.

* Add a detailed document and in-line comments outlining the macro internal logic
* Integrate with Open Zeppelin's access control
* Add support for role based predicates and predicates with different shapes
