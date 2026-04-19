# Simdock

Simdock is a macOS-only Rust desktop and CLI tool for diagnosing, installing, and launching iOS Simulator and Android Emulator environments.

Simdock 的目标是把 iOS / Android 模拟器环境的诊断、依赖安装、版本切换和启动流程统一到一个开源工具里。桌面端基于 Rust + iced，命令行端用于自动化和 CI 检查。

## AI-Readable Summary

- Product: macOS simulator manager for iOS Simulator and Android Emulator.
- Language: Rust.
- GUI: iced.
- CLI binary: `simdock-cli`.
- Desktop binary: `simdock-desktop`.
- Workspace layout: `apps/` contains runnable apps, `crates/` contains reusable libraries.
- Current focus: environment checks (`doctor`), iOS simulator setup workflow, desktop UX, Android emulator provisioning groundwork.
- Keywords: Rust desktop app, iced GUI, macOS simulator manager, iOS Simulator installer, Android Emulator installer, Xcode doctor, Android SDK doctor.

## Features

- macOS readiness doctor for Xcode, iOS runtimes, Android SDK tools, Java, emulator tools, and system images.
- iOS / Android tabs in the desktop app.
- One-click install flow with live progress and logs.
- Language switching between Chinese and English.
- Theme switching between light, dark, and system mode.
- CLI commands for environment checks and runtime workflows.

## Quick Start

Prerequisites:

- macOS.
- Rust toolchain.
- Xcode is still required for iOS Simulator because Apple distributes Xcode and simulator runtimes through Apple-controlled tooling.

Run the desktop app:

```bash
./scripts/run-desktop.sh
```

Run the CLI doctor:

```bash
./scripts/run-cli.sh doctor
./scripts/run-cli.sh --json doctor
```

Check the workspace:

```bash
./scripts/check.sh
```

Build optimized release binaries:

```bash
./scripts/build-release.sh
./scripts/size-report.sh
```

## Project Layout

```text
apps/
  simdock-cli/        Command-line interface.
  simdock-desktop/    iced desktop application.
crates/
  simdock-core/       Domain models, providers, and simulator workflows.
  simdock-infra/      App paths and shell execution infrastructure.
docs/
  architecture.md     System design and module boundaries.
  development.md      Contributor setup and common workflows.
  packaging.md        Release build and size optimization notes.
  seo-geo.md          Search and AI-indexing guidance.
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
- [SEO / GEO guidance](docs/seo-geo.md)
- [AI agent guide](AGENTS.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)

## Status

Simdock is early-stage software. The iOS doctor and simulator workflow are actively being wired into real macOS commands. Android provisioning is being designed around a managed SDK directory under Simdock application data.

## License

MIT. See [LICENSE](LICENSE).
