# Contributing

This package is under active development, and contributions are welcome. Planned and potential future work is tracked in the [TODOS](./CONTRIBUTORS.md#todos) section.

To contribute, please open a small, focused pull request with an accompanying test. Before committing, ensure the codebase is properly formatted by running:

```bash
make fmt
make clippy
```

If you change the macro instrumentation logic or add a new attribute macro, include:

* a positive test showing the injected guard runs
* a negative test proving a missing attribute is caught
* A PR description to illustrate the motivation and transformation

Please keep the macro behavior predictable and the error messages short and actionable.

## TODOS

The following improvements/additions to the macro are in the pipeline.

* Integrate with Open Zeppelin's access control
* Add support for role based predicates and predicates with different shapes
* Add some example use-cases which leverage multiple auth predicates to show its flexibility, and utility
* Strengthen the test suite
