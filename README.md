# VECTRUM

**Compatibility intelligence for evolving software systems.**

VECTRUM compares contract snapshots, classifies structural compatibility changes, traces consumers through a declared dependency graph, and emits machine-readable evidence.

## Quick start

```bash
cargo run -- info
cargo run -- compare --baseline examples/baseline.json --candidate examples/candidate.json --graph examples/dependencies.json
```

Fail a CI job when a breaking change is detected:

```bash
cargo run -- compare --baseline old.json --candidate new.json --fail-on-breaking
```

Write a report:

```bash
cargo run -- compare --baseline old.json --candidate new.json --output vectrum-report.json
```

## Current capabilities

- Rust library and CLI.
- JSON, JSON Schema-like JSON, and OpenAPI JSON normalization.
- Structural additions, removals, and modifications.
- Conservative compatibility classification.
- Optional transitive dependency impact analysis.
- JSON reports with before/after values, evidence, and explicit limitations.
- CI workflow and regression tests.

## Compatibility semantics

VECTRUM distinguishes breaking, compatible, and review-required changes. It deliberately classifies unknown modifications as review-required rather than claiming safety without evidence.

## Boundaries

This release does not parse YAML, Protobuf, or SQL DDL. It does not resolve all external schema references, inspect source code to discover dependencies, execute services, or prove runtime behavior. See docs/SCOPE.md.

## Development

```bash
cargo fmt --all
cargo test --all-targets
cargo build --release
```

## License

Apache-2.0.
