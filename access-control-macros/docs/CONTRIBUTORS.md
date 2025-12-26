# Contributing

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
* Always import soroban_sdk::Env into the file when this macro is used. Any name collisions with `Env` should cause a compilation error. This is because `Env` can be also declared in some other crates apart from `soroban_sdk` which can be problematic.
* Integrate with Open Zeppelin's access control
* Add support for role based predicates and predicates with different shapes
* Eliminate pub(crate) Fns from the enforced functions
