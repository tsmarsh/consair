# Release Guide

This document explains how to create releases for Consair.

## CI/CD Workflows

### CI Workflow (`ci.yml`)

Runs automatically on every push to `main`/`master` and on pull requests.

**Jobs:**
- **test**: Runs all tests on Linux, Windows, and macOS
- **clippy**: Checks code quality with clippy
- **fmt**: Verifies code formatting
- **build**: Builds release binaries for all platforms

### Release Workflow (`release.yml`)

Runs automatically when you push a version tag (e.g., `v0.1.0`).

**Platforms:**
- Linux x86_64
- Linux ARM64
- macOS x86_64 (Intel)
- macOS ARM64 (Apple Silicon)
- Windows x86_64

**Artifacts:**
- `cons-linux-x86_64` - Linux x86_64 binary
- `cons-linux-aarch64` - Linux ARM64 binary
- `cons-macos-x86_64` - macOS Intel binary
- `cons-macos-aarch64` - macOS Apple Silicon binary
- `cons-windows-x86_64.exe` - Windows binary

## Creating a Release

### 1. Update Version

Update the version in `Cargo.toml`:

```toml
[package]
version = "0.2.0"  # Update this
```

### 2. Commit Changes

```bash
git add Cargo.toml
git commit -m "Bump version to 0.2.0"
```

### 3. Create and Push Tag

```bash
# Create annotated tag
git tag -a v0.2.0 -m "Release version 0.2.0"

# Push tag to GitHub
git push origin v0.2.0
```

### 4. Wait for GitHub Actions

The release workflow will automatically:
1. Create a GitHub release
2. Build binaries for all platforms
3. Upload binaries to the release

### 5. Edit Release Notes (Optional)

Visit the [Releases](../../releases) page and edit the release to add:
- Description of changes
- Breaking changes
- Bug fixes
- New features

## Testing Locally

### Build for your platform

```bash
cargo build --release
```

The binary will be at `target/release/cons` (or `cons.exe` on Windows).

### Test the binary

```bash
# Run the REPL
./target/release/cons

# Or with cargo
cargo run --release
```

## Cross-compilation

### Linux → Windows

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

### Linux/macOS → Other platforms

Use [`cross`](https://github.com/cross-rs/cross):

```bash
cargo install cross --git https://github.com/cross-rs/cross

# Build for ARM64 Linux
cross build --release --target aarch64-unknown-linux-gnu

# Build for other targets
cross build --release --target x86_64-pc-windows-gnu
```

## Troubleshooting

### Build fails on macOS ARM64

Make sure you have the target installed:

```bash
rustup target add aarch64-apple-darwin
```

### Release workflow fails

Check that:
1. The tag follows the format `v*` (e.g., `v0.1.0`, `v1.2.3`)
2. You have pushed the tag: `git push origin <tag-name>`
3. GitHub Actions are enabled in your repository settings

### Binary is too large

The release builds are already optimized. You can further reduce size with:

```bash
# In Cargo.toml
[profile.release]
strip = true
lto = true
codegen-units = 1
panic = "abort"
```

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (1.0.0): Breaking changes
- **MINOR** (0.1.0): New features, backwards compatible
- **PATCH** (0.0.1): Bug fixes, backwards compatible

Examples:
- `v0.1.0` - Initial release
- `v0.1.1` - Bug fix
- `v0.2.0` - New feature (add arithmetic operations)
- `v1.0.0` - Stable API, production ready
