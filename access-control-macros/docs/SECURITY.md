# Security considerations

While this macro helps prevent *accidental* access-control omissions, it does not eliminate all security risks. Developers and auditors should be aware of the following considerations.

## Predicate correctness is critical

Authorization predicates are user-defined and not validated by the macro. A flawed predicate (for example, checking the wrong storage key or using incorrect comparison logic) will still compile and execute.

The macro guarantees *that* a predicate is called, not *that the predicate is correct*.

## State-dependent predicates

Predicates often depend on on-chain state (e.g. owner stored in contract storage). If that state can be modified within the same transaction or reentered through other calls, developers must reason carefully about ordering and reentrancy.

The macro does not enforce reentrancy protection or state immutability.

### Initialization and front-running

The macro does not protect initialization logic. If ownership or privileged roles are set via an unprotected initializer, the contract may still be vulnerable to front-running at deployment time.

Developers should use constructors or atomic deployment patterns where appropriate.

### TTL and storage lifecycle

Predicates commonly read from persistent storage. If TTLs are not extended correctly, critical authorization state may expire unexpectedly, leading to denial-of-service or unintended access changes.

TTL management is explicitly left to the contract author.

### Macro scope and visibility

The macro only instruments functions within the annotated `impl`. Functions outside this scope, including helpers or generated wrappers, are not protected unless explicitly guarded.

Auditors should verify that all external entrypoints are covered by `#[access_control]`.

# Limitations

This macro is purposefully small and opinionated by design. It only instruments functions inside annotated `impl` block and does not operate on code generated elsewhere (for example, client stubs or wrappers emitted by other macros). It scans “public” methods—trait impls, anything in an `impl` also tagged with `#[contractimpl]`, or any method with non-private visibility, and requires each `pub` method to be marked `#[no_access_control]` or `#[authorized_by(...)]`.

In `#[access_control]` `impl` blocks, the procedural macro will cause a compiler error if the access-controlled method signature
does not include an `env` parameter named `env`, if no parameter matches the identifier specified
in `#[authorized_by]`, or if that parameter does not support `require_auth()`.
However, `#[authorized_by]` only functions correctly if inside a `#[access_control]` `impl` block.
If the outer macro is not present, the inner attribute is left in place and the macro quietly skips instrumentation.

For predicate resolution, a single-segment name is invoked as `Self::predicate`, otherwise the path is used as written. The macro calls it before `require_auth()`. The predicate should only be used to perform the intended verification, and should not modify any state. The macro does not **currently** provide role composition, multi-sig policies, or reentrancy protection, and it doesn’t rewrite or verify the logic inside your predicate.

Interacting proc-macros can still confuse IDEs. We bias toward being non-fatal for unresolved shapes to avoid rust-analyzer spam, but you may see stale diagnostics until a clean build. Finally, `#[access_control]` must be placed on an inline `impl` (or inline `mod`). External modules are rejected because their contents aren’t visible at macro time.

## What this macro does not do

This macro does not attempt to:

- Verify the correctness of predicate logic
- Enforce role hierarchies or policy composition
- Implement direct access control
- Prevent misuse of `require_auth()` outside the macro

It is meant to be a **guardrail**, and not a full-fledged security framework.
