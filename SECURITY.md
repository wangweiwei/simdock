# Security Policy

Simdock executes local macOS tools such as `xcodebuild`, `xcrun`, `simctl`, Android SDK tools, and emulator binaries. Treat command construction and privilege boundaries carefully.

## Reporting a Vulnerability

Open a private security advisory if the repository host supports it. If not, contact the maintainers through the project issue tracker and avoid posting exploit details publicly until a maintainer responds.

## Security Guidelines

- Do not execute shell strings when a structured command with explicit arguments is possible.
- Never accept untrusted input as a program path without validation.
- Ask for explicit user confirmation before privileged macOS operations.
- Keep downloaded SDK artifacts inside Simdock-managed directories unless the user chooses otherwise.
- Do not log secrets, local tokens, or credential paths.

## Supported Versions

Simdock is pre-1.0. Security fixes target the main branch until a stable release channel exists.
