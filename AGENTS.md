# AI Agent Guide

This repository is friendly to AI-assisted development, but agents must preserve project boundaries and user trust.

## Repository Map

- `apps/simdock-cli`: CLI entry points and automation-friendly commands.
- `apps/simdock-desktop`: iced UI, app state, styling, and localization.
- `crates/simdock-core`: simulator domain models, providers, environment checks, and install workflows.
- `crates/simdock-infra`: filesystem paths and command-running infrastructure.
- `docs`: contributor, architecture, packaging, and search/AI indexing docs.
- `scripts`: stable contributor commands.

## Agent Rules

- Run `./scripts/check.sh` before declaring code complete when feasible.
- Keep user-facing GUI text in `apps/simdock-desktop/src/i18n.rs`.
- Prefer small, reviewable changes over broad rewrites.
- Add Chinese doc comments to core public APIs and complex workflow functions.
- Do not add network downloads or privileged commands without documenting the trust boundary.
- Do not remove macOS-only assumptions unless the architecture explicitly changes.

## Common Tasks

- Add a new environment check in `crates/simdock-core/src/provider`.
- Add GUI text in `apps/simdock-desktop/src/i18n.rs`.
- Add CLI behavior in `apps/simdock-cli/src/main.rs`.
- Update contributor docs in `docs/development.md`.

## Verification

```bash
./scripts/check.sh
./scripts/run-cli.sh doctor
./scripts/run-desktop.sh
```
