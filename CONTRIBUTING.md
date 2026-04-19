# Contributing

Thanks for helping improve Simdock.

## Development Setup

1. Install Rust on macOS.
2. Clone the repository.
3. Run the workspace checks:

```bash
./scripts/check.sh
```

4. Start the desktop app:

```bash
./scripts/run-desktop.sh
```

5. Run CLI commands:

```bash
./scripts/run-cli.sh doctor
```

## Code Standards

- Keep domain logic in `crates/simdock-core`.
- Keep OS paths and process execution helpers in `crates/simdock-infra`.
- Keep UI state and rendering in `apps/simdock-desktop`.
- Keep automation-friendly commands in `apps/simdock-cli`.
- Add Chinese doc comments for core public APIs and non-obvious workflow steps.
- Avoid adding new runtime dependencies unless they clearly reduce risk or complexity.

## Pull Request Checklist

- `./scripts/check.sh` passes.
- User-facing text is routed through `apps/simdock-desktop/src/i18n.rs` when used by the GUI.
- New simulator workflow steps emit progress/log events.
- Documentation is updated when behavior changes.
- macOS-specific behavior is described explicitly.

## Commit Style

Use concise imperative commit subjects, for example:

```text
Add iOS runtime environment checks
Move desktop app into apps workspace
Document release size profile
```
