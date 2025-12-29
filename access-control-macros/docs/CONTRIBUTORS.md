# Contributing

This package is under active development, and developer contributions are welcome. To contribute to the library, open a small PR with a focused change and a matching test. Before committing, run formatting locally with:

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

The following improvements/additions to the macro are in the pipeline. Developers willing to contribute can pick up any of 
these items to work on.

* Integrate with Open Zeppelin's access control
* Add support for role based predicates and predicates with different shapes
* Add some example use-cases which leverage multiple auth predicates to show its flexibility, and utility
