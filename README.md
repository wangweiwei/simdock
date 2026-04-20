<p align="center">
  <img src="assets/brand/png/simdock-app-icon-256.png" alt="Simdock logo" width="112" height="112">
</p>

<h1 align="center">Simdock</h1>

<p align="center">
  <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-0D747A?style=flat-square">
  <img alt="Platform" src="https://img.shields.io/badge/platform-macOS-111820?style=flat-square">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2024-CB5A44?style=flat-square">
  <img alt="GUI" src="https://img.shields.io/badge/GUI-iced-2B87FF?style=flat-square">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-6F7B86?style=flat-square">
  <a href="README.zh-CN.md"><img alt="Language" src="https://img.shields.io/badge/language-English-0D747A?style=flat-square"></a>
</p>

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

## Screenshots

Screenshots are not committed yet. Store release-ready PNG screenshots under `assets/screenshots/`, then reference them from this section with relative Markdown image paths.

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

Build a macOS `.app` bundle and `.dmg` with Simdock icons:

```bash
./scripts/package-macos.sh
```

Build a macOS `.pkg` installer:

```bash
./scripts/package-macos-pkg.sh
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
  package-macos.sh    macOS app bundle and DMG packaging helper.
  package-macos-pkg.sh
                      macOS PKG installer packaging helper.
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

Simdock is still early-stage software. The current focus is improving iOS/Android environment checks, managed iOS simulator setup, managed Android SDK/AVD setup, emulator launch workflows, and the desktop experience.

## License

MIT. See [LICENSE](LICENSE).
