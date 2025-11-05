# Advisory

This project is experimental and unaudited. Use at your own risk and review generated code and unit tests before deploying to mainnet.

# Overview

In Sororban, it is common to forget to validate access control properly. This is because there is no global `msg.sender` and instead an `Address` is passed as an input into the methods for performing any validations. Any time a privileged action is taken one must both verify the address is relevant (for example that it’s the `owner`) and prove the caller actually authorized the invocation with `require_auth()`. These two steps are easy to miss and apply consistently as the number of functions and/or contracts within a project grow.

This crate adds a tiny, declarative access-control layer so you can state intent and let a proc-macro enforce it. Put #[access_control] on your impl block, then mark each public entrypoint as either explicitly open with #[no_access_control] or protected with #[authorized_by(arg, predicate)]. The macro injects both the predicate check and the require_auth() call, and it fails the build if any public function is missing either one of these annotations.

# Usage

```
// The access control macro will be applied to this contract implementation.
#[access_control]
#[contractimpl]
impl MyContract {
    // No access control required, so no guard injected
    #[no_access_control]
    pub fn balance_of(env: Env, user: Address) -> u128 { ... }

    // Protected endpoint — guard is injected at the top of the body:
    //   if !Self::only_owner(&env, &caller) { panic!("unauthorized") }
    //   caller.require_auth();
    #[authorized_by(caller, only_owner)]
    pub fn change_owner(env: Env, caller: Address, new_owner: Address) {
        env.storage().persistent().set(&DataKey::Owner, &new_owner);
    }
}

impl MyContract {
    // The predicate used by authorized_by() to enforce acess control
    fn only_owner(env: &Env, who: &Address) -> bool {
        env.storage().persistent().get::<_, Address>(&DataKey::Owner)
            .map_or(false, |owner| &owner == who)
    }
}
```

## Details

## Details

### What the macros do (and what code they inject)

When you tag an `impl` with `#[access_control]`, the macro:

1. **Finds every method** in that `impl`.
2. For each method tagged `#[authorized_by(arg, predicate_path)]`, it **injects a guard** at the very top of the function body and then **removes** the `#[authorized_by(..)]` attribute so downstream macros won’t see it.

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

3. For each public-ish method (public visibility **or** in a trait impl **or** inside an impl that will be passed to `#[contractimpl]`), it **enforces** that the method is either:

   * marked `#[no_access_control]` (explicitly open), or
   * marked `#[authorized_by(..)]` (protected).

If neither is present, the build fails with a clear error telling you which method needs an annotation.

> “Public-ish” means: `pub`/`pub(crate)`/`pub(super)`/`pub(in …)`, **or** the `impl` is a trait impl, **or** the `impl` carries `#[contractimpl]` (since those become external entrypoints).

### Recommended attribute order (multiple macros)

Put `#[access_control]` **above** `#[contractimpl]`:

```rust
#[access_control]
#[contractimpl]
impl MyContract { /* methods */ }
```

* With this order, `access_control` instruments your methods **first**, strips `#[authorized_by(..)]`, and hands a clean, already-guarded impl to `#[contractimpl]`.
* If you reverse the order, `contractimpl` might synthesize wrappers and the original `#[authorized_by(..)]` could land on a non-function item. To avoid noisy analyzer errors, the standalone `#[authorized_by]` attribute in this crate is tolerant: if it doesn’t see a function/method shape (or required params), it simply leaves the item unchanged and warns at most. Still, the **recommended** order is `#[access_control]` then `#[contractimpl]`.

### Predicates: what they must look like

Write your predicate to be **pure** and **side-effect free**: no storage writes, no auth calls, and no non-determinism. The macro expects the signature:

```rust
fn predicate(env: &Env, who: &Address) -> bool
```

It can be:

* an inherent method on the same type (e.g. `fn only_owner(&Env, &Address) -> bool`), referenced as `only_owner` (the macro rewrites to `Self::only_owner`), or
* any path like `crate::auth::is_admin`.

**Do not** call `require_auth()` inside the predicate; the macro injects that **after** the predicate check passes. Predicates should answer only “is this address allowed, given current on-chain state?”

### External modules are ignored (by design)

`#[access_control]` works on:

* an `impl` block, or
* an **inline** `mod` (the content is present in the same file).

If you put it on an **external** module (declared with `mod x;` and defined elsewhere), the macro cannot inspect the contents. In that case it raises a hard error: use it directly on the `impl` (or inline the module).

### How we avoid conflicts with `#[contractimpl]`

* **In-place editing**: `access_control` uses `syn` to parse your `impl` and **replaces** each annotated method’s block with a guarded block (the snippet shown above). It then **removes** the `#[authorized_by(..)]` attribute so `contractimpl` never sees it.
* **Graceful fallbacks**: If a method doesn’t have both required parameters (`env` and the named `arg`), the instrumentation simply **skips** that method (and `access_control` still enforces the “every public-ish fn must be annotated” rule). This prevents rust-analyzer from spamming errors on generated wrappers while still catching real misses at build time.
* **Clear diagnostics**: If a function is public-ish but lacks `#[no_access_control]` or `#[authorized_by(..)]`, we emit a focused error naming that function. If an `#[authorized_by(..)]` references a missing parameter (e.g., `sender` not in the signature), we warn and leave the body unchanged; `access_control` will then complain about the missing protection on that entrypoint, which points you to the underlying fix.

### Before/after example

Source:

```rust
impl MyContract {
    // open entrypoint
    #[no_access_control]
    pub fn ping(env: Env) { /* ... */ }

    // protected entrypoint
    #[authorized_by(user, only_owner)]
    pub fn bump(env: Env, user: Address, n: u32) {
        /* original body */
    }

    // predicate (pure; read-only)
    fn only_owner(env: &Env, user: &Address) -> bool {
        let owner: Option<Address> = env.storage().persistent().get(&DataKey::Owner);
        matches!(owner, Some(ref o) if o == user)
    }
}
```

After `#[access_control]`:

```rust
impl MyContract {
    pub fn ping(env: Env) { /* unchanged */ }

    pub fn bump(env: Env, user: Address, n: u32) {
        if !(Self::only_owner(&env, &user)) {
            ::core::panic!("unauthorized: only_owner(env,user) failed");
        }
        user.require_auth();
        /* original body */
    }

    fn only_owner(env: &Env, user: &Address) -> bool { /* unchanged */ }
}
```

### Referencing predicates by path

You can point to a predicate outside the impl:

```rust
#[authorized_by(caller, crate::auth::is_admin)]
pub fn rotate_keys(env: Env, caller: Address) { /* ... */ }
```

As long as `crate::auth::is_admin(&Env, &Address) -> bool` exists and is pure, the guard injects cleanly.

### What happens if you forget an annotation?

If a public-ish function in the `impl` has **no** `#[no_access_control]` and **no** `#[authorized_by(..)]`, compilation fails with:

```
public method <name> is missing #[no_access_control] or #[authorized_by(...)]
```

Add the appropriate attribute and rebuild.

### A note on generated client methods

Methods created by `#[contractimpl]` (client shims) are **not** places to put `#[authorized_by]`; our macro strips that attribute **before** `contractimpl` runs specifically to avoid forwarding it. Always annotate the **original** methods in your impl; the client wrappers will invoke your now-instrumented bodies.

### Example: end-to-end

```rust
#[access_control]
#[contractimpl]
impl Banking {
    #[no_access_control]
    pub fn version(_env: Env) -> u32 { 1 }

    #[authorized_by(caller, only_owner)]
    pub fn set_fee(env: Env, caller: Address, bps: i64) {
        // Guard is injected here:
        //   if !(Self::only_owner(&env, &caller)) panic!("unauthorized");
        //   caller.require_auth();
        // Then your logic:
        save_fee(&env, bps);
    }

    fn only_owner(env: &Env, who: &Address) -> bool {
        get_owner(env).as_ref() == Some(who)
    }
}
```

Place `#[access_control]` **above** `#[contractimpl]`, keep predicates pure, and mark every public-ish entrypoint as **open** or **protected**. The macro handles the rest—injecting checks, ordering safely with `contractimpl`, ignoring external modules, and surfacing crisp compile-time errors when something’s missing.

## Limitations

This macro is purposefully small and opinionated by design. It only instruments functions inside annotated `impl` block and does not operate on code generated elsewhere (for example, client stubs or wrappers emitted by other macros). It scans “public” methods—trait impls, anything in an `impl` also tagged with `#[contractimpl]`, or any method with non-private visibility, and requires each `pub` method to be marked `#[no_access_control]` or `#[authorized_by(...)]`.

The procedural macro only injects a guard when the method signature includes an `env` parameter named `env`, plus a simple (non-destructured) parameter whose identifier matches the first argument to `#[authorized_by]`. If either is missing or renamed, the attribute is left in place and the macro quietly skips instrumentation. The guarded parameter must support `require_auth()` (on Soroban that’s `Address`). If any type does not support it, the injected call will fail to compile.

For predicate resolution, a single-segment name is invoked as `Self::predicate`, otherwise the path is used as written. The macro calls it before `require_auth()`. The predicate should only be used to perform the intended verification, and should not modify any state. The macro does not currently provide role composition, multi-sig policies, or reentrancy protection, and it doesn’t rewrite or verify the logic inside your predicate.

Interacting proc-macros can still confuse IDEs. We bias toward being non-fatal for unresolved shapes to avoid rust-analyzer spam, but you may see stale diagnostics until a clean build. Finally, `#[access_control]` must be placed on an inline `impl` (or inline `mod`). External modules are rejected because their contents aren’t visible at macro time.

# Developers

## Building

To build the contracts you need only a couple prerequisites:

* A recent stable [Rust](https://www.rust-lang.org/) toolchain
* An editor that supports Rust
* [Stellar CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup#install-the-stellar-cli)

See the [documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) for more prerequisite installation instructions.

The stellar contracts can then be built with ```make build```.

## Testing

The repository also contains some tests which demonstrate the macros in action. These can be run with ```make test```.

## Contributing

Open a small PR with a focused change and a matching test. Run formatting and lints locally:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

If you change instrumentation logic, include:

* a positive test showing the injected guard runs
* a negative test proving a missing attribute is caught
* A PR description to illustrate the transformation

Please keep the macro behavior predictable and the error messages short and actionable.

## TODOS

The following improvements to the macro are in the pipeline.

* Integrate with Open Zeppelin's access control or role-based access control for the predicate
