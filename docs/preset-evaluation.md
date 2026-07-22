# Preset Evaluation

Evaluation is a deterministic, offline library API owned by the `model-routing`
crate. It is exercised by Rust tests and maintainer release verification; v0.3.1
does not expose a standalone evaluation command.

The report records the evaluation suite id, suite hash, bundle hash, scenario count, status, and recommendation state. Offline evaluation cannot claim live verification or recommendation status.

Recommended status requires later authenticated host evidence and signature verification.
