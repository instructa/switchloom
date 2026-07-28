# Switchloom

**Model routing for coding agents, without hand-writing provider config.**

Switchloom turns a small routing policy into repository-local agent roles. You
choose which models handle implementation, review, and verification; Switchloom
generates the native configuration for your coding agent.

[**Build a routing setup →**](https://switchloom.ai)

```text
your coding agent (orchestrator)
├── implementer  → fast or cost-efficient model
├── reviewer     → stronger reasoning model
└── verifier     → lightweight verification model
```

For native agent teams, the model selected in your coding agent remains the
orchestrator. Pi uses that active session to delegate through Pi Subagents.

## Why Switchloom

- **Use the right model for each job.** Keep routine implementation and checks
  on efficient models while reserving stronger reasoning for review.
- **Start from a useful preset.** Light, Balanced, and High provide practical
  defaults; every child role remains editable.
- **Keep setup local to the repository.** Switchloom writes project-native
  configuration without changing global agent settings.
- **Review every change.** Preview shows the exact files before apply, and the
  lifecycle supports status, update, rollback, and uninstall.
- **Use one workflow across hosts.** Codex, Claude Code, Cursor, OpenCode, and Pi
  share the same routing policy while receiving their own native artifacts.

## Quick start

The easiest path is the generator at [switchloom.ai](https://switchloom.ai).
Choose a host and preset, copy the generated command, and run it inside the
repository you want to configure:

```sh
npx switchloom apply --recipe 'sw1_...' --repository .
npx switchloom doctor codex
```

`apply` previews the change and asks before writing. `doctor` checks the local
host installation and explains any configuration or reload issue.

To work directly from the CLI:

```sh
npm install --global switchloom

switchloom compile balanced --host codex-openai --output routing-bundle.json
switchloom preview routing-bundle.json --repository .
switchloom apply routing-bundle.json --repository .
switchloom doctor codex
```

Homebrew is also supported:

```sh
brew install instructa/tap/switchloom
```

## Presets and roles

| Preset | Best for |
| --- | --- |
| Light | Small changes and low-cost routine work |
| Balanced | Everyday implementation with independent review and verification |
| High | Important changes where stronger review matters more than cost |

Presets are starting points, not locked bundles. You can change a role's model
or reasoning effort, or remove roles you do not need.

## Repository lifecycle

```sh
switchloom status --repository .
switchloom update routing-bundle.json --repository .
switchloom rollback --repository .
switchloom uninstall --repository .
```

Switchloom tracks only the files it manages. It refuses unsafe paths and
preserves unrelated repository configuration.

## Supported hosts

- Codex
- Claude Code
- Cursor
- OpenCode
- Pi

Standalone use is the default. Planr integration is optional and does not add a
runtime dependency on Planr.

## What Switchloom does not do

Switchloom compiles and applies routing configuration. It does not replace your
coding agent, make model calls itself, control provider billing, or guarantee
that a custom model combination will be cheaper or better. Custom setups should
be reviewed and tested in the target repository.

## Documentation

- [Routing policy](docs/model-routing-policy.md)
- [Presets and repository lifecycle](docs/preset-composition.md)
- [Model and preset evaluation](docs/preset-evaluation.md)
- [Package and repository boundaries](docs/package-policy.md)
- [Changelog](CHANGELOG.md)

## License

MIT. See [LICENSE](LICENSE).
