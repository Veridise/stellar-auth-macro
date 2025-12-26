## Limitations

This macro is purposefully small and opinionated by design. It only instruments functions inside annotated `impl` block and does not operate on code generated elsewhere (for example, client stubs or wrappers emitted by other macros). It scans “public” methods—trait impls, anything in an `impl` also tagged with `#[contractimpl]`, or any method with non-private visibility, and requires each `pub` method to be marked `#[no_access_control]` or `#[authorized_by(...)]`.

In `#[access_control]` `impl` blocks, the procedural macro will cause a compiler error if the access-controlled method signature
does not include an `env` parameter named `env`, if no parameter matches the identifier specified
in `#[authorized_by]`, or if that parameter does not support `require_auth()`.
However, `#[authorized_by]` only functions correctly if inside a `#[access_control]` `impl` block.
If the outer macro is not present, the inner attribute is left in place and the macro quietly skips instrumentation.

For predicate resolution, a single-segment name is invoked as `Self::predicate`, otherwise the path is used as written. The macro calls it before `require_auth()`. The predicate should only be used to perform the intended verification, and should not modify any state. The macro does not **currently** provide role composition, multi-sig policies, or reentrancy protection, and it doesn’t rewrite or verify the logic inside your predicate.

Interacting proc-macros can still confuse IDEs. We bias toward being non-fatal for unresolved shapes to avoid rust-analyzer spam, but you may see stale diagnostics until a clean build. Finally, `#[access_control]` must be placed on an inline `impl` (or inline `mod`). External modules are rejected because their contents aren’t visible at macro time.