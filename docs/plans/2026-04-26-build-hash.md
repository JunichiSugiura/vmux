# Build Hash Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Differentiate build profiles (release, local, dev) with distinct app names, bundle IDs, and auto-update eligibility.

**Architecture:** A `build.rs` embeds the git short hash at compile time. The `VMUX_PROFILE` environment variable (`release` | `local` | `dev`) controls the app name and bundle ID. A packaging script patches `Cargo.toml` metadata, `Info.plist`, and `inject-cef.sh` before running `cargo packager`, then restores originals. Dev mode uses the existing `debug` feature flag.

**Tech Stack:** Rust build.rs, Make, Bash

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/vmux_desktop/build.rs` | Create | Embed git hash + profile as compile-time env vars |
| `crates/vmux_desktop/src/lib.rs` | Modify | Set window title based on profile |
| `crates/vmux_desktop/src/main.rs` | Modify | Show profile + hash in startup banner |
| `scripts/package.sh` | Create | Profile-aware packaging (patches metadata, runs cargo packager, restores) |
| `scripts/inject-cef.sh` | Modify | Read bundle ID from env var instead of hardcoding |
| `Makefile` | Modify | Wire up profile-aware targets |

---

### Task 1: Add build.rs to embed git hash and profile

**Files:**
- Create: `crates/vmux_desktop/build.rs`

- [ ] **Step 1: Create build.rs**

```rust
// crates/vmux_desktop/build.rs
use std::process::Command;

fn main() {
    // Git short hash (7 chars)
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo::rustc-env=VMUX_GIT_HASH={hash}");

    // Profile: VMUX_PROFILE env var, default to "dev" for debug builds, "release" for release builds
    let profile = std::env::var("VMUX_PROFILE").unwrap_or_else(|_| {
        if std::env::var("PROFILE").unwrap_or_default() == "release" {
            "release".to_string()
        } else {
            "dev".to_string()
        }
    });
    println!("cargo::rustc-env=VMUX_PROFILE={profile}");

    // Rebuild when HEAD changes or VMUX_PROFILE changes
    println!("cargo::rerun-if-changed=../../.git/HEAD");
    println!("cargo::rerun-if-changed=../../.git/refs");
    println!("cargo::rerun-if-env-changed=VMUX_PROFILE");
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p vmux_desktop --features debug 2>&1 | tail -5`
Expected: successful build, no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/vmux_desktop/build.rs
git commit -m "feat: add build.rs to embed git hash and profile"
```

---

### Task 2: Set window title and startup banner based on profile

**Files:**
- Modify: `crates/vmux_desktop/src/lib.rs:42-53` (window title)
- Modify: `crates/vmux_desktop/src/main.rs:20-21` (startup banner)

- [ ] **Step 1: Add window title based on profile in lib.rs**

In `crates/vmux_desktop/src/lib.rs`, modify the `build` method of `VmuxPlugin` to set the window title dynamically. Replace the `primary_window` construction:

```rust
        let title = match env!("VMUX_PROFILE") {
            "release" => "Vmux".to_string(),
            "local" => format!("Vmux ({})", env!("VMUX_GIT_HASH")),
            "dev" => "Vmux Dev".to_string(),
            other => format!("Vmux ({})", other),
        };

        let primary_window = NativeWindow {
            title,
            transparent: true,
            composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
            decorations: true,
            titlebar_shown: false,
            movable_by_window_background: false,
            fullsize_content_view: true,
            ..default()
        };
```

- [ ] **Step 2: Update startup banner in main.rs**

In `crates/vmux_desktop/src/main.rs`, update the version line in the println banner. Replace:

```rust
         \x1b[2mv{}\x1b[0m\n",
        env!("CARGO_PKG_VERSION")
```

with:

```rust
         \x1b[2mv{}{}\x1b[0m\n",
        env!("CARGO_PKG_VERSION"),
        match env!("VMUX_PROFILE") {
            "release" => String::new(),
            "local" => format!(" ({})", env!("VMUX_GIT_HASH")),
            "dev" => " dev".to_string(),
            other => format!(" ({})", other),
        }
```

- [ ] **Step 3: Verify it compiles and shows correct output**

Run: `cargo build -p vmux_desktop --features debug 2>&1 | tail -5`
Expected: successful build. The dev profile should be the default for debug builds.

- [ ] **Step 4: Commit**

```bash
git add crates/vmux_desktop/src/lib.rs crates/vmux_desktop/src/main.rs
git commit -m "feat: set window title and banner based on build profile"
```

---

### Task 3: Make inject-cef.sh read bundle ID from environment

**Files:**
- Modify: `scripts/inject-cef.sh`

- [ ] **Step 1: Replace hardcoded BUNDLE_ID_BASE with env var fallback**

In `scripts/inject-cef.sh`, change line:

```bash
BUNDLE_ID_BASE="ai.vmux.desktop"
```

to:

```bash
BUNDLE_ID_BASE="${VMUX_BUNDLE_ID:-ai.vmux.desktop}"
```

Also change:

```bash
APP_BUNDLE="${ROOT}/target/release/Vmux.app"
```

to:

```bash
APP_BUNDLE="${VMUX_APP_BUNDLE:-${ROOT}/target/release/Vmux.app}"
```

- [ ] **Step 2: Commit**

```bash
git add scripts/inject-cef.sh
git commit -m "feat: make inject-cef.sh bundle ID configurable via env"
```

---

### Task 4: Create scripts/package.sh for profile-aware packaging

**Files:**
- Create: `scripts/package.sh`

- [ ] **Step 1: Create the packaging script**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Profile-aware packaging for Vmux.
#
# Usage:
#   ./scripts/package.sh              # defaults to "local"
#   ./scripts/package.sh release
#   ./scripts/package.sh local
#
# Patches Cargo.toml packager metadata and Info.plist for the target profile,
# runs cargo packager, then restores the originals.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${1:-local}"

CARGO_TOML="$ROOT/crates/vmux_desktop/Cargo.toml"
INFO_PLIST="$ROOT/packaging/macos/Info.plist"

case "$PROFILE" in
    release)
        PRODUCT_NAME="Vmux"
        BUNDLE_ID="ai.vmux.desktop"
        ;;
    local)
        GIT_HASH=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
        PRODUCT_NAME="Vmux ($GIT_HASH)"
        BUNDLE_ID="ai.vmux.desktop.local"
        ;;
    *)
        echo "Unknown profile: $PROFILE (expected: release, local)" >&2
        exit 1
        ;;
esac

echo "==> Packaging profile: $PROFILE"
echo "    Product name: $PRODUCT_NAME"
echo "    Bundle ID:    $BUNDLE_ID"

# Backup originals
cp "$CARGO_TOML" "$CARGO_TOML.bak"
cp "$INFO_PLIST" "$INFO_PLIST.bak"

restore() {
    mv "$CARGO_TOML.bak" "$CARGO_TOML"
    mv "$INFO_PLIST.bak" "$INFO_PLIST"
}
trap restore EXIT

# Patch Cargo.toml packager metadata
sed -i '' "s/^product-name = .*/product-name = \"$PRODUCT_NAME\"/" "$CARGO_TOML"
sed -i '' "s/^identifier = .*/identifier = \"$BUNDLE_ID\"/" "$CARGO_TOML"

# Patch Info.plist
sed -i '' "s|<string>ai\.vmux\.desktop</string>|<string>$BUNDLE_ID</string>|" "$INFO_PLIST"
# Update display name (the line after CFBundleDisplayName)
sed -i '' "/<key>CFBundleDisplayName<\/key>/{n;s|<string>.*</string>|<string>$PRODUCT_NAME</string>|;}" "$INFO_PLIST"
# Update bundle name (the line after CFBundleName)
sed -i '' "/<key>CFBundleName<\/key>/{n;s|<string>.*</string>|<string>$PRODUCT_NAME</string>|;}" "$INFO_PLIST"

# Export for inject-cef.sh
export VMUX_BUNDLE_ID="$BUNDLE_ID"
export VMUX_PROFILE="$PROFILE"

# The app bundle path uses the product name from packager metadata.
# cargo-packager always outputs to target/release/<product-name>.app
# For "Vmux (abc1234)" this becomes "target/release/Vmux (abc1234).app"
# inject-cef.sh needs to know the correct path.
APP_NAME="$PRODUCT_NAME"
export VMUX_APP_BUNDLE="$ROOT/target/release/$APP_NAME.app"

echo "==> Running cargo packager"
cd "$ROOT"
env -u CEF_PATH VMUX_PROFILE="$PROFILE" cargo packager --release

echo "==> Packaging complete: $VMUX_APP_BUNDLE"
```

- [ ] **Step 2: Make it executable**

Run: `chmod +x scripts/package.sh`

- [ ] **Step 3: Commit**

```bash
git add scripts/package.sh
git commit -m "feat: add profile-aware packaging script"
```

---

### Task 5: Update Makefile with profile-aware targets

**Files:**
- Modify: `Makefile`

- [ ] **Step 1: Update Makefile targets**

Replace the existing `package-mac`, `build-local-mac`, and `run-mac-local` targets and add a `package-release-mac` target:

```makefile
# Replace:
run-mac-local: package-mac
	open target/release/Vmux.app

package-mac:
	env -u CEF_PATH cargo packager --release

build-local-mac: package-mac
	@echo "Signing..."
	SKIP_NOTARIZE=1 ./scripts/sign-and-notarize.sh

# With:
run-mac-local: package-local-mac
	@HASH=$$(git rev-parse --short HEAD 2>/dev/null || echo unknown); \
	open "target/release/Vmux ($$HASH).app"

package-local-mac:
	./scripts/package.sh local

package-release-mac:
	./scripts/package.sh release

build-local-mac: package-local-mac
	@echo "Signing..."
	@HASH=$$(git rev-parse --short HEAD 2>/dev/null || echo unknown); \
	APP_BUNDLE="target/release/Vmux ($$HASH).app" SKIP_NOTARIZE=1 ./scripts/sign-and-notarize.sh

build-release-mac: package-release-mac
	@echo "Signing..."
	./scripts/sign-and-notarize.sh
```

Also update the `.PHONY` line to include the new targets:

```makefile
.PHONY: run-mac run-mac-local run-doctor build-mac-debug build build-local-mac build-release-mac package-local-mac package-release-mac setup-cef install-debug-render-process doctor-mac ensure-run-mac-deps run-website build-website
```

- [ ] **Step 2: Verify Makefile syntax**

Run: `make -n package-local-mac`
Expected: shows the commands that would run without executing them.

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "feat: add profile-aware packaging targets to Makefile"
```

---

### Task 6: Verify end-to-end

- [ ] **Step 1: Verify dev profile (debug build)**

Run: `cargo build -p vmux_desktop --features debug 2>&1 | tail -3`
Expected: compiles with `VMUX_PROFILE=dev` and `VMUX_GIT_HASH=<hash>`.

- [ ] **Step 2: Verify local profile env var override**

Run: `VMUX_PROFILE=local cargo build -p vmux_desktop --features debug 2>&1 | tail -3`
Expected: compiles with `VMUX_PROFILE=local`.

- [ ] **Step 3: Verify inject-cef.sh accepts env var**

Run: `grep 'VMUX_BUNDLE_ID' scripts/inject-cef.sh`
Expected: shows `BUNDLE_ID_BASE="${VMUX_BUNDLE_ID:-ai.vmux.desktop}"`.

- [ ] **Step 4: Verify package.sh dry run**

Run: `head -20 scripts/package.sh && echo "---" && bash -n scripts/package.sh && echo "Syntax OK"`
Expected: script syntax is valid.
