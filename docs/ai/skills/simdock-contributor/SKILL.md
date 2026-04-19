# Simdock Contributor Skill

Use this skill when contributing code, documentation, or release tooling to Simdock.

## Workflow

1. Read `AGENTS.md` and `docs/architecture.md`.
2. Identify whether the change belongs in `apps/` or `crates/`.
3. Keep desktop UI strings in `apps/simdock-desktop/src/i18n.rs`.
4. Add Chinese doc comments for core public APIs or complex simulator workflow steps.
5. Run `./scripts/check.sh`.

## Boundaries

- Do not move simulator business logic into the iced view layer.
- Do not introduce privileged commands without user confirmation and documentation.
- Do not claim that Simdock bundles Xcode.
- Keep macOS assumptions explicit.

## Useful Commands

```bash
./scripts/check.sh
./scripts/run-cli.sh doctor
./scripts/run-desktop.sh
./scripts/size-report.sh
```
