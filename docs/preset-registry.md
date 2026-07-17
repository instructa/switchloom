# Preset Registry

Switchloom can generate and verify catalog data for the CLI and public website from the same package-owned compiler path:

```sh
model-routing catalog build --output website/data/catalog.json
model-routing catalog verify website/data/catalog.json
```

Detached Ed25519 signatures bind catalog content to a trusted signer:

```sh
model-routing registry sign website/data/catalog.json --signer maintainers --private-key-file key.hex --output catalog.sig.json
model-routing registry verify website/data/catalog.json --signature catalog.sig.json --trusted-signer maintainers --trusted-public-key-file public.hex
```

Unsigned catalog entries remain experimental.

The public catalog exposes host, preset or custom topology, role model, effort, fork behavior, usage profile, registry state, and optional Planr integration controls. Official and experimental entries come from repository-owned inputs; signed and recommended states require trusted signatures and current evaluation evidence. Custom or unverified entries should be treated as local/user-owned until verified.
