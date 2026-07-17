# Model Routing Policy

Switchloom owns opinionated model selection and host bindings for supported agent hosts. The standalone compiler emits `RoutingBundle v1` JSON with deterministic profiles, routes, artifacts, hashes, and evidence labels.

Standalone compilation is the default:

```sh
model-routing compile balanced --host codex-openai --output routing-bundle.json
```

The default bundle contains repository-local host artifacts only. Optional Planr integration is explicit:

```sh
model-routing compile balanced --host codex-openai --integration planr --output routing-bundle.json
```

Inspect validates a bundle and emits a machine-readable summary:

```sh
model-routing inspect routing-bundle.json
```

Offline evaluations remain `experimental` until authenticated live-host evidence and a maintainer signature are available.
