# Preset Evaluation

Evaluation commands are reproducible and offline:

```sh
model-routing evaluate balanced --host codex-openai
```

The report records the evaluation suite id, suite hash, bundle hash, scenario count, status, and recommendation state. Offline evaluation cannot claim live verification or recommendation status.

Recommended status requires later authenticated host evidence and signature verification.
