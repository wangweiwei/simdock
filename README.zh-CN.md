# Simdock

[English](README.md)

Simdock 是一个仅面向 macOS 的开源工具，用 Rust 构建，提供桌面应用和 CLI，用于统一管理 iOS Simulator 与 Android Emulator 的环境检测、依赖安装和启动流程。

桌面端基于 `iced`，命令行端用于自动化、脚本集成和贡献者调试。

## 功能特性

- 检测 macOS 是否已具备运行 iOS 模拟器和 Android 模拟器的基础环境。
- 检查 Xcode、iOS 运行时、Android SDK 工具、Java、emulator、ADB 和系统镜像。
- 桌面端支持 iOS / Android Tab 切换。
- 提供一键安装流程、当前步骤、进度和实时日志。
- 支持中文 / English 语言切换。
- 支持浅色、深色、跟随系统主题。
- 提供 CLI 命令，方便自动化和后续 CI 集成。

## 快速开始

环境要求：

- macOS。
- Rust 工具链。
- iOS 模拟器仍需要 Xcode。Xcode 本体由 Apple 分发，Simdock 不会内置或绕过 Apple 的分发和授权机制。

运行桌面应用：

```bash
./scripts/run-desktop.sh
```

运行 CLI 环境检测：

```bash
./scripts/run-cli.sh doctor
./scripts/run-cli.sh --json doctor
```

检查整个 workspace：

```bash
./scripts/check.sh
```

构建 release 版本并查看体积：

```bash
./scripts/build-release.sh
./scripts/size-report.sh
```

## 项目结构

```text
apps/
  simdock-cli/        命令行应用。
  simdock-desktop/    基于 iced 的桌面应用。
crates/
  simdock-core/       领域模型、Provider 和模拟器工作流。
  simdock-infra/      应用目录、命令执行等基础设施。
docs/
  architecture.md     架构和模块边界。
  development.md      贡献者开发说明。
  packaging.md        构建、发布和体积优化说明。
scripts/
  check.sh            格式化和编译检查。
  run-cli.sh          CLI 开发运行脚本。
  run-desktop.sh      桌面端开发运行脚本。
  build-release.sh    release 构建脚本。
  size-report.sh      release 体积报告脚本。
```

## 文档

- [架构说明](docs/architecture.md)
- [开发说明](docs/development.md)
- [打包和体积优化](docs/packaging.md)
- [AI Agent 协作说明](AGENTS.md)
- [贡献指南](CONTRIBUTING.md)
- [安全策略](SECURITY.md)

## 当前状态

Simdock 仍处于早期阶段。当前重点是完善 iOS / Android 环境检测、iOS 模拟器安装启动流程、桌面端体验，以及 Android 托管 SDK / AVD 的后续安装能力。

## License

MIT。详见 [LICENSE](LICENSE)。
