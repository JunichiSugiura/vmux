//! Helpers for `build.rs` scripts that produce Dioxus web `dist/` folders via **`dx build`**.
//!
//! Requires [**`dioxus-cli`**](https://crates.io/crates/dioxus-cli) on `PATH` (or the **`DX`** env var).
//! The CLI vendors Tailwind, `wasm-bindgen`, and related tooling — no Node/npm for these bundles.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;

/// Workspace root (parent of `crates/`) from a member crate manifest dir.
pub fn workspace_root_from_manifest_dir(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("vmux crates should live under workspace crates/")
        .to_path_buf()
}

fn dx_version_ok(path: &Path) -> bool {
    Command::new(path)
        .arg("--version")
        .status()
        .is_ok_and(|s| s.success())
}

/// Resolve the `dx` executable: **`DX`** env, then `PATH`, then `~/.cargo/bin/dx`.
pub fn resolve_dx_executable() -> PathBuf {
    if let Ok(dx) = std::env::var("DX") {
        let p = PathBuf::from(dx);
        if dx_version_ok(&p) {
            return p;
        }
    }
    if dx_version_ok(Path::new("dx")) {
        return PathBuf::from("dx");
    }
    if let Some(home) = std::env::var_os("HOME") {
        let p = PathBuf::from(home).join(".cargo/bin/dx");
        if dx_version_ok(&p) {
            return p;
        }
    }
    panic!(
        "vmux: `dx` (dioxus-cli) not found. Install e.g.\n\
         cargo install dioxus-cli --locked --version 0.7.4\n\
         Or set DX=/path/to/dx"
    );
}

/// `target/dx/<bin_name>/{debug|release}/web/public` after `dx build --platform web`.
pub fn dx_web_public_dir(workspace_root: &Path, bin_name: &str, release: bool) -> PathBuf {
    let profile = if release { "release" } else { "debug" };
    workspace_root
        .join("target")
        .join("dx")
        .join(bin_name)
        .join(profile)
        .join("web")
        .join("public")
}

/// Run `dx build --platform web -p <package>` from the workspace root.
pub fn run_dx_web_bundle(
    workspace_root: &Path,
    package: &str,
    release: bool,
    extra_dx_args: &[&str],
) {
    let dx = resolve_dx_executable();
    let mut cmd = Command::new(&dx);
    cmd.current_dir(workspace_root)
        .env_remove("CEF_PATH")
        .args(["build", "--platform", "web", "-p", package]);
    if release {
        cmd.arg("--release");
    }
    for a in extra_dx_args {
        cmd.arg(a);
    }
    let status = cmd.status().unwrap_or_else(|e| {
        panic!(
            "vmux: failed to spawn dx ({}): {e}",
            dx.display()
        )
    });
    if !status.success() {
        panic!(
            "vmux: `dx build --platform web -p {package}` failed (release={release})"
        );
    }
}

/// Replace `dist/` with `dx`’s `public/` output, then write the CEF **`shell_index.html`**
/// with **`__VMUX_DX_ENTRY__`** / **`__VMUX_DX_WASM__`** filled from `dx`’s generated `index.html`
/// (hashed `assets/` paths or `wasm/` layout — both are handled).
pub fn replace_dist_from_dx_public(public: &Path, dist: &Path, shell_index: &Path) {
    if !public.is_dir() {
        panic!(
            "vmux: expected dx web output at {} (directory missing after dx build)",
            public.display()
        );
    }
    if dist.exists() {
        fs::remove_dir_all(dist).unwrap_or_else(|e| {
            panic!("vmux: failed to remove {}: {e}", dist.display());
        });
    }
    fs::create_dir_all(dist).unwrap_or_else(|e| {
        panic!("vmux: failed to create {}: {e}", dist.display());
    });
    copy_dir_recursive(public, dist).unwrap_or_else(|e| {
        panic!(
            "vmux: copy {} -> {}: {e}",
            public.display(),
            dist.display()
        );
    });
    merge_cef_shell_index(dist, shell_index);
}

fn merge_cef_shell_index(dist: &Path, shell_index: &Path) {
    let dx_html = fs::read_to_string(dist.join("index.html")).unwrap_or_else(|e| {
        panic!("vmux: read dx index.html in {}: {e}", dist.display());
    });
    let entry_href = dx_module_script_href(&dx_html);
    let wasm_href = find_bg_wasm_href(dist);

    let shell = fs::read_to_string(shell_index).unwrap_or_else(|e| {
        panic!("vmux: read shell {}: {e}", shell_index.display());
    });
    let merged = shell
        .replace("__VMUX_DX_ENTRY__", &entry_href)
        .replace("__VMUX_DX_WASM__", &wasm_href);
    if merged.contains("__VMUX_DX_ENTRY__") || merged.contains("__VMUX_DX_WASM__") {
        panic!(
            "vmux: shell {} is missing __VMUX_DX_ENTRY__ / __VMUX_DX_WASM__ placeholders",
            shell_index.display()
        );
    }
    fs::write(dist.join("index.html"), merged).unwrap_or_else(|e| {
        panic!("vmux: write merged index.html: {e}");
    });
}

fn dx_module_script_href(dx_html: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<script\s+[^>]*type="module"[^>]*\ssrc="([^"]+)""#)
            .expect("regex")
    });
    let Some(cap) = RE.captures(dx_html) else {
        panic!("vmux: dx index.html has no <script type=\"module\" src=\"...\">");
    };
    href_for_shell(&cap[1])
}

fn href_for_shell(dx_url: &str) -> String {
    let path = dx_url
        .trim_start_matches("/./")
        .trim_start_matches('/');
    format!("./{path}")
}

fn find_bg_wasm_href(dist: &Path) -> String {
    let wasm_dir = dist.join("wasm");
    if wasm_dir.is_dir() {
        let Ok(rd) = fs::read_dir(&wasm_dir) else {
            panic!("vmux: read {}", wasm_dir.display());
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with("_bg.wasm") {
                return format!("./wasm/{name}");
            }
        }
    }
    let assets = dist.join("assets");
    if assets.is_dir() {
        let Ok(rd) = fs::read_dir(&assets) else {
            panic!("vmux: read {}", assets.display());
        };
        let mut picks: Vec<String> = Vec::new();
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with("_bg.wasm") || name.contains("_bg-") && name.ends_with(".wasm") {
                picks.push(name);
            }
        }
        picks.sort();
        if let Some(name) = picks.into_iter().next() {
            return format!("./assets/{name}");
        }
    }
    panic!(
        "vmux: no *_bg*.wasm under {}/wasm or {}/assets",
        dist.display(),
        dist.display()
    );
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            fs::create_dir_all(&dest)?;
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}
