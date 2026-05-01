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

## Rules

- Do not add comments to code.
- Do not use mod.rs files. Use the filename-based module pattern (e.g. `layout.rs` + `layout/` directory).

## Linear

When taking a Linear issue (e.g. "take VMX-XX"), immediately move it to **In Progress** before doing anything else — before creating a worktree, before reading code, before drafting a PR.

## Worktrees

**Never edit files on the main worktree.** All changes must happen inside a feature worktree. Before writing any code for a Linear issue:

1. Check if a worktree already exists: `git worktree list`
2. Create worktree if needed: `git worktree add .worktrees/vmx-<number> -b <branch-name>` — always name the worktree directory using the `vmx-<number>` convention matching the Linear issue (e.g., `.worktrees/vmx-88`).
3. `cd` into the worktree directory and make all edits there.
4. When done, merge to main, then remove: `git worktree remove .worktrees/<short-name>`
5. Remember: if the worktree is deleted while your shell is inside it, `cd` back to the repo root — `../..` won't work.

Worktree directory: `.worktrees/` (already in `.gitignore`).

## Documentation

- Save design specs to `docs/specs/YYYY-MM-DD-<topic>-design.md` (not `docs/superpowers/specs/`).
- Save implementation plans to `docs/plans/YYYY-MM-DD-<feature-name>.md` (not `docs/superpowers/plans/`).
- Delete the plan file once the plan is fully implemented.

## Git

Always prefer `git rebase` over `git merge` when updating branches. Use `git push --force-with-lease` after rebasing.

## Before Pushing / Opening PRs

**Mandatory**: Run `make lint` and `make test` before every `git push` or PR creation. Do not push or open a PR if either command fails. Fix all errors first.

```sh
make lint      # fmt --check + clippy -D warnings (excludes vendored patches)
make test      # cargo test --workspace (excludes bevy_cef_core)
make lint-fix  # auto-fix: runs fmt + clippy --fix
```

If `make lint` fails with formatting errors, run `make lint-fix` to auto-format, then verify with `make lint` again.
