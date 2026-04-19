# Packaging and Size Optimization

Simdock currently produces two binaries:

- `simdock-cli`
- `simdock-desktop`

## Release Build

```bash
./scripts/build-release.sh
```

## Size Report

```bash
./scripts/size-report.sh
```

Current local baseline:

```text
simdock-cli: 860K
simdock-desktop: 4.0M
```

## Current Release Profile

The workspace release profile is configured for smaller binaries:

- `opt-level = "z"` optimizes for size.
- `lto = "thin"` enables link-time optimization with reasonable build time.
- `codegen-units = 1` improves optimization opportunities.
- `panic = "abort"` avoids unwinding machinery.
- `strip = "symbols"` removes symbols from release binaries.

## Future Size Work

- Audit GUI dependencies with `cargo tree -p simdock-desktop`.
- Consider feature-gating Android download code once provisioning lands.
- Avoid bundling assets that macOS already provides, especially fonts.
- Keep CLI independent from iced so automation installs stay small.
