# Simdock

[中文](README.zh-CN.md)

Simdock is a macOS-only open-source tool written in Rust. It provides both a desktop app and a CLI for managing iOS Simulator and Android Emulator environment checks, dependency setup, and launch workflows.

The desktop app is built with `iced`. The CLI is designed for automation, scripting, and contributor debugging.

## Features

- Check whether a Mac is ready to run iOS Simulator and Android Emulator.
- Inspect Xcode, iOS runtimes, Android SDK tools, Java, emulator, ADB, and system images.
- Switch between iOS and Android tabs in the desktop app.
- Run one-click setup workflows with current step, progress, and live logs.
- Switch between Chinese and English.
- Switch between light, dark, and system themes.
- Use CLI commands for automation and future CI integration.

## Quick Start

Requirements:

- macOS.
- Rust toolchain.
- Xcode is still required for iOS Simulator. Xcode is distributed by Apple, and Simdock does not bundle or bypass Apple's distribution and license mechanisms.

Run the desktop app:

```bash
./scripts/run-desktop.sh
```

Run the CLI environment check:

```bash
./scripts/run-cli.sh doctor
./scripts/run-cli.sh --json doctor
```

Check the workspace:

```bash
./scripts/check.sh
```

Build optimized release binaries and inspect size:

```bash
./scripts/build-release.sh
./scripts/size-report.sh
```

## Project Layout

```text
apps/
  simdock-cli/        Command-line application.
  simdock-desktop/    iced desktop application.
crates/
  simdock-core/       Domain models, providers, and simulator workflows.
  simdock-infra/      App paths, command execution, and infrastructure.
docs/
  architecture.md     Architecture and module boundaries.
  development.md      Contributor development guide.
  packaging.md        Build, release, and size optimization notes.
scripts/
  check.sh            Format and compile checks.
  run-cli.sh          CLI development runner.
  run-desktop.sh      Desktop development runner.
  build-release.sh    Release build helper.
  size-report.sh      Release binary size helper.
```

## Documentation

- [Architecture](docs/architecture.md)
- [Development](docs/development.md)
- [Packaging and size optimization](docs/packaging.md)
- [AI agent guide](AGENTS.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)

## Status

Simdock is still early-stage software. The current focus is improving iOS / Android environment checks, the iOS simulator setup and launch workflow, the desktop experience, and the future managed Android SDK / AVD installation flow.

## License

MIT. See [LICENSE](LICENSE).
