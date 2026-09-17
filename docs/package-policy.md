# Package and release policy

The npm package contains metadata, README, LICENSE, the launcher and supported
native binaries with provenance. The Cargo source package contains the Rust
implementation, embedded `catalog.toml` and the repository's Switchloom skill.
The skill is installed from the checkout; it is not another native installer.
Research clones, local host state,
credentials, databases, receipts and build output remain excluded.

The CLI provides `route` for task decisions, `handoff` for prepared desktop
dispatch arguments, plus `status` and `uninstall` for existing installations.
Version 1.0.0 is prepared locally and not yet published. Live verification
remains a release requirement. The website makes no TypeSafe calls and accepts
no API key. The desktop skill uses the installed CLI and native Codex tools.

## Generated task data

`cargo run -p xtask -- release prepare --allow-dirty` regenerates
`website/data/catalog.json` from `catalog.toml`, including model suggestions,
supported reasoning and model defaults and capabilities. Astro imports that catalog at
build time. There are no public bundle downloads or install recipes. Catalog
verification checks that the generated file matches its owner.

## Release ownership

`scripts/release.sh <version> "summary"` guards branch/remote state, invokes
preparation and verification, then performs the requested commit/tag/push.
A dry run performs preparation and verification but skips publishing actions.
`xtask release` owns version consistency, clean-source requirements, package
inventories and native provenance. `generate-formula.sh` is used by the
conditional Homebrew tap job.

Only the product crate is publishable. The existing `model-routing` native
binary identity and version output are shared by the npm launcher and native
provenance. Both `switchloom` and `model-routing` remain published entry points.

Use Node 24 for repository development and deployment, matching CI. The npm
launcher has its separate Node 18 minimum. `deploy:test` and `destroy:test`
invoke Alchemy directly. Native artifacts, security checks and website/package
publishing belong to release verification, not ordinary local test runs.
