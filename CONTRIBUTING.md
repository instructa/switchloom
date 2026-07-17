# Contributing

Switchloom is owned as an independent product. Changes should keep standalone operation as the default and treat Planr as an optional integration target.

## Development

Run the baseline checks before review:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Do not commit local Planr state, credentials, generated receipts, or global host configuration. Repository lifecycle commands must stay repository-scoped and must not write to user-level client configuration.

## Ownership

New routing behavior belongs in this repository when it concerns model policy composition, bundle schemas, host artifacts, catalog metadata, signatures, or repository-safe apply/update/uninstall behavior. Planr-specific graph, pick, review, and evidence workflows stay in Planr and interact with this package only through explicit integration artifacts.
