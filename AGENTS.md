# Agent Instructions

## Communication Style

Use caveman mode. Terse, direct, no filler. Execute first, talk second. No meta-commentary, no preamble, no postamble. Code speaks.

## Skills

Use superpower. Invoke relevant skills BEFORE any response or action. Even a 1% chance a skill might apply means invoke it.

## Pre-commit Checks

NEVER commit or push without running these checks locally first and confirming they pass:

```bash
# 1. Format check (all workspace packages, excluding patches)
PKGS=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.manifest_path | test("patches") | not) | .name')
for pkg in $PKGS; do
  cargo fmt -p "$pkg" -- --check
done

# 2. Clippy (all workspace packages, excluding patches)
for pkg in $PKGS; do
  env -u CEF_PATH cargo clippy -p "$pkg" --all-targets -- -D warnings
done

# 3. Tests
env -u CEF_PATH cargo test --workspace --exclude bevy_cef_core
```

If any check fails, fix the issue before committing. Do not push broken code.

## Platform-Specific Code

This project targets macOS (primary) and Linux (CI). When adding imports or code that uses platform-specific APIs (CEF, winit, AppKit), always add appropriate `#[cfg(...)]` gates. Run `cargo fmt` after adding cfg-gated imports -- rustfmt reorders them.
