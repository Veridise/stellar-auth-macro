# Details

## What the macros do (and what code they inject)

When you tag an `impl` with `#[access_control]`, the macro walks every method in that block and inspects its attributes. For each method marked `#[authorized_by(arg, predicate_path)]`, it rewrites the function by injecting a guard at the very start of the body and then strips the `#[authorized_by(..)]` attribute so downstream macros never see it. Everything else is left as-is unless you add other annotations.

Note: `#[authorized_by]` is syntax-only unless consumed by `#[access_control]`.

Injected guard (conceptually):

```rust
{
    // Enforce that `env` is actually soroban_sdk::Env
    let _: &soroban_sdk::Env = &env;

    if !(predicate_path(&env, &arg)) {
        ::core::panic!("unauthorized: one or more authorization predicates failed");
    }
    arg.require_auth();
    /* original body follows */
}
```

For each public method (public visibility **or** in a trait `impl` **or** inside an `impl` that will be passed to `#[contractimpl]`), the macro **enforces** that the method is either:

* marked `#[no_access_control]` (explicitly open), or
* marked `#[authorized_by(..)]` (protected).

If neither is present, the build fails with a clear error telling you which method needs an annotation.

> The macro enforces annotations on each public endpoint. Any method in a trait impl, any method in an impl tagged #[contractimpl], or any method declared pub. Methods with restricted visibility like pub(crate), pub(super), or pub(in …) are not enforced (unless the impl is #[contractimpl] or a trait impl). 

### Recommended attribute order (multiple macros)

Put `#[access_control]` **above** `#[contractimpl]`:

```rust
#[access_control]
#[contractimpl]
impl MyContract { /* methods */ }
```

With this order, `access_control` instruments your methods **first**, strips `#[authorized_by(..)]`, and hands a clean, already-guarded `impl` to `#[contractimpl]`.If you reverse the order, #[contractimpl] may generate wrappers/stubs before #[access_control] runs, and the access-control macro may no longer see or rewrite the original method bodies as intended.

To avoid noisy analyzer errors, the standalone `#[authorized_by]` attribute in this crate is tolerant: if it doesn’t see a function/method shape (or required params), it simply leaves the item unchanged and warns at most. Still, the **recommended** order is `#[access_control]` then `#[contractimpl]`.

## Predicates: recommended shape and behavior

Predicates are expected to be kept **pure, deterministic, and read-only**.

They should not:

* Modify contract storage
* Call `require_auth()`, as the macro already handles that for the concerned addresses
* Perform non-deterministic operations

The expected predicate signature is:

```rust
fn predicate(env: &Env, who: &Address) -> bool
```

It can be:

* an inherent method on the same type (e.g. `fn only_owner(&Env, &Address) -> bool`), referenced as `only_owner` (the macro rewrites to `Self::only_owner`), or
* any path like `crate::auth::is_admin`.

Calling `require_auth()` inside the predicate is *not* necessary; the macro injects that **after** the predicate check passes. Typically, predicates should answer only “is this address allowed, given the current on-chain state?”

## Macro expansion: Before and After

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
    fn only_owner(env: &Env, user: &Address) -> bool {
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
        if !(Self::only_owner(&env, &user)) {
            ::core::panic!("unauthorized: only_owner(env,user) failed");
        }
        user.require_auth();
        /* body */
    }

    fn only_owner(env: &Env, user: &Address) -> bool { /* unchanged */ }
}
```

## Referencing predicates by path

You can point to a predicate outside the `impl` using:

```rust
#[authorized_by(caller, crate::auth::is_admin)]
pub fn transfer_fees(env: Env, caller: Address) { /* ... */ }
```

As long as `crate::auth::is_admin(&Env, &Address) -> bool` exists, the guard will inject cleanly.

## What happens if you forget an annotation?

If a public function in the `impl` has **no** `#[no_access_control]` and **no** `#[authorized_by(..)]`, compilation fails with:

```plaintext
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
        matches!(owner, Some(ref o) if o == who)
    }
}
```

Place `#[access_control]` **above** `#[contractimpl]`, keep predicates read-only, and mark every public entrypoint as **#[no_access_control]** or **#[authorized_by()]**. The macro handles the rest, injecting checks, ordering safely with `contractimpl`, ignoring external modules, and surfacing compile-time errors when something’s missing.
