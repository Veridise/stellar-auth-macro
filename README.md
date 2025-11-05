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
