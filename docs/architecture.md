# Architecture

Simdock uses a small Rust workspace split by responsibility.

## Layers

`apps/simdock-cli`

The CLI exposes automation-friendly commands. It should remain thin: parse arguments, call `simdock-core`, and print results.

`apps/simdock-desktop`

The desktop app owns UI state, iced widgets, styling, localization, and event wiring. It should not contain simulator business logic beyond presentation-specific state.

`crates/simdock-core`

The core crate owns the domain model and simulator workflows. Providers implement platform-specific behavior for iOS and Android.

`crates/simdock-infra`

The infra crate owns local filesystem paths and shell execution abstractions. It is the right place for reusable OS integration helpers.

## Provider Model

Each platform provider implements:

- `doctor`: inspect whether the platform is ready.
- `list_runtimes`: list installed or available runtimes.
- `list_device_templates`: list supported device templates.
- `install_runtime`: install or prepare the runtime and emit task events.
- `create_profile`: create a launch profile.
- `start`: start a simulator instance.
- `stop`: stop a simulator instance.

## Event Flow

Install workflows emit `TaskEvent` values. CLI callers can stream or print them later; the desktop app maps them into progress, current step, and live logs.

## macOS Boundary

Simdock currently targets macOS only. iOS depends on Apple tooling (`Xcode.app`, `xcodebuild`, `xcrun`, `simctl`). Android is designed around a managed SDK directory under the user's application support path.
