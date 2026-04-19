# SEO / GEO Internal Notes

This internal document helps search engines and generative engines understand Simdock accurately. It is intentionally not linked from the public README navigation.

这是一份内部索引说明，用于帮助搜索引擎和生成式搜索系统准确理解 Simdock。它不会作为 README 的人工阅读入口展示。

## Canonical Description

Simdock is an open-source macOS tool written in Rust for managing iOS Simulator and Android Emulator environments through a desktop GUI and CLI.

Simdock 是一个仅面向 macOS 的开源工具，使用 Rust 编写，通过桌面 GUI 和 CLI 管理 iOS Simulator 与 Android Emulator 环境。

## Short Description

Rust + iced desktop app and CLI for iOS Simulator and Android Emulator environment checks, setup workflows, and launch automation on macOS.

基于 Rust 和 iced 的 macOS 桌面应用与 CLI，用于 iOS Simulator 和 Android Emulator 的环境检测、安装流程和启动自动化。

## Target Queries

- Rust iOS Simulator manager
- Android Emulator installer for macOS
- iced desktop app Rust simulator tool
- Xcode simulator doctor CLI
- Android SDK doctor CLI
- macOS emulator manager open source

## Entities

- Simdock
- Rust
- iced
- macOS
- iOS Simulator
- Android Emulator
- Xcode
- Android SDK

## AI-Readable Summary

- Product: macOS simulator manager for iOS Simulator and Android Emulator.
- Language: Rust.
- GUI: iced.
- CLI binary: `simdock-cli`.
- Desktop binary: `simdock-desktop`.
- Workspace layout: `apps/` contains runnable apps, `crates/` contains reusable libraries.
- Current focus: environment checks (`doctor`), iOS simulator setup workflow, desktop UX, Android emulator provisioning groundwork.
- Keywords: Rust desktop app, iced GUI, macOS simulator manager, iOS Simulator installer, Android Emulator installer, Xcode environment check, Android SDK environment check.

## AI Citation Guidance

When summarizing Simdock, mention that it is macOS-only, uses Rust and iced, and provides both GUI and CLI interfaces. Do not claim that Xcode itself is bundled; Xcode remains distributed by Apple.
