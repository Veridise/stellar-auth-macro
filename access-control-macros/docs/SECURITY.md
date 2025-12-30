# Security considerations

While this macro helps prevent *accidental* access-control omissions, it does not eliminate all security risks. Developers and reviewers should be aware of the following considerations.

## Predicate correctness is critical

Authorization predicates are user-defined and not validated by the macro. A flawed predicate (for example, checking the wrong storage key or using incorrect comparison logic) will still compile and execute.

The macro guarantees *that* a predicate is called, not *that the predicate is correct*.

### Initialization and front-running

The macro does not protect initialization logic. If ownership or privileged roles are set via an unprotected initializer, the contract may still be vulnerable to front-running at deployment time.

Developers should use constructors or atomic deployment patterns where appropriate.

### TTL and storage lifecycle

Predicates commonly read from persistent storage. If TTLs are not extended correctly, critical authorization state may expire unexpectedly, leading to denial-of-service or unintended access changes.

TTL management is explicitly left to the contract author.

### Macro scope and visibility

The macro only instruments functions within the annotated `impl`. Functions outside this scope, including helpers or generated wrappers, are not protected unless explicitly guarded.

Developers should verify that all external entrypoints are covered by `#[access_control]`.

# Limitations

This macro is intentionally small, explicit, and opinionated by design. It only instruments functions defined directly inside an annotated `impl` block (or an inline `mod`) and does not operate on code generated elsewhere, such as client stubs or wrappers emitted by other procedural macros. Because attribute macros only receive the token stream of the item they are attached to, the macro cannot traverse the crate, inspect external modules, or rewrite code across module boundaries. As a result, enforcement is local to the annotated scope.

Within a `#[access_control]` impl, the macro scans the public surface of the contract methods in trait implementations, methods inside an impl tagged with `#[contractimpl]`, and methods with public visibility, and requires each such method to be explicitly marked with either `#[no_access_control]` or `#[authorized_by(...)]`. If a public method is missing both annotations, compilation fails with a clear error. Methods that are not part of the public surface (for e.g. `pub(crate)` or private helpers) are ignored.

For instrumentation to succeed, an access-controlled method must include a parameter named exactly `env` whose type is `Env` or `soroban_sdk::Env` (optionally by reference). The identifier referenced in `#[authorized_by(arg, …)]` must exist in the function signature and must support `require_auth()`. If these requirements are not met, the macro skips instrumentation and emits a warning rather than failing hard. The `#[authorized_by]` attribute has no effect unless it appears inside a `#[access_control]` `impl` block. When used outside that context, it is left in place and silently ignored.

Authorization predicates are invoked before `require_auth()` and are expected to be deterministic and read-only. The macro does not inspect, rewrite, or verify predicate logic, nor does it enforce role hierarchies, policy composition, multisig authorization, or reentrancy protection. These concerns are intentionally left to the contract author. Finally, interacting procedural macros can still confuse IDE tooling such as rust-analyzer. While the macro biases toward being non-fatal for unresolved shapes to reduce diagnostic noise, stale warnings may appear until a clean build is performed.

## What this macro does not do

This macro does not attempt to:

- Verify the correctness of predicate logic
- Enforce role hierarchies or policy composition
- Implement access control primitives itself
- Prevent misuse of `require_auth()` outside the macro

It is meant to be a **guardrail**, and not a full-fledged authorization framework.
