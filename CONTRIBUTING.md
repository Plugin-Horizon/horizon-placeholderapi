# Contributing to horizon-placeholderapi

Pull requests and improvements are always welcome! Bug reports, ideas and documentation fixes count just as much as code.

## Building and testing the core

The core crate is plain Rust with no dependencies and builds without the Horizon engine, so you only need a stable Rust toolchain:

- `core`

```sh
(cd core && cargo test)
```

CI runs exactly these commands (job name `test`) on every push and pull request. The trait mod's glue (`api/src/lib.rs`) depends on the Horizon engine and is not built by CI; if you change it, say in the PR how you tested the change.

## Parsing changes

The parsing rules are PlaceholderAPI's. A change to the scanner, the relational pass or the registry should come with a test that states the rule and, where it can, the upstream behaviour it follows (the PlaceholderAPI source file and the commit).

## Workflow

1. Fork the repository (or create a branch if you have write access) and branch from `main`.
2. Make your change. Keep it focused, and add or update tests where it makes sense.
3. Run the tests locally.
4. Open a pull request against `main` and fill in the template.
5. One approving review is needed before merging. Direct pushes to `main` are disabled.

## Licence of contributions

By submitting a contribution you agree that it is licensed under the same licence as this repository (see the `LICENSE` file, GPL-3.0-or-later). Only submit code you wrote yourself or that comes from a source whose licence is compatible; mention the source in the code.

## Be nice

See CODE_OF_CONDUCT.md.
