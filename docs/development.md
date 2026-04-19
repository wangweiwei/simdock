# Development

## Requirements

- macOS.
- Rust toolchain.
- Xcode for iOS simulator workflows.

## First Run

```bash
./scripts/check.sh
./scripts/run-cli.sh doctor
./scripts/run-desktop.sh
```

## Useful Commands

Format and compile:

```bash
./scripts/check.sh
```

Run CLI:

```bash
./scripts/run-cli.sh doctor
./scripts/run-cli.sh --json doctor
./scripts/run-cli.sh runtime list --platform ios
```

Run desktop:

```bash
./scripts/run-desktop.sh
```

Build release:

```bash
./scripts/build-release.sh
```

Measure release binaries:

```bash
./scripts/size-report.sh
```

## Localization

Desktop UI text belongs in `apps/simdock-desktop/src/i18n.rs`. Do not scatter language checks through the view layer.

## Documentation Comments

Core APIs should explain intent in Chinese because the current maintainer workflow is Chinese-first. Keep comments short and focused on behavior, assumptions, and safety boundaries.
