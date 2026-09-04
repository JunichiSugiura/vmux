#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="${HOME}/.cargo/bin:${PATH}"
unset CEF_PATH

APP_BUNDLE="${APP_BUNDLE:-$ROOT/target/release/Vmux ($(git -C "$ROOT" rev-parse --short HEAD)).app}"

usage() {
	cat <<'EOF'
Usage: macos.sh [command]

  dev       Build, sign, and run target/debug/vmux_desktop. Default.
  bundle    Build, sign, and open Vmux (<sha>).app.
  help      Show this help

Environment (optional):
  APP_BUNDLE  Full path to .app when opening after bundle (default: target/release/Vmux (<sha>).app)
EOF
}

cmd_dev() {
	make -C "$ROOT" dev
}

cmd_bundle() {
	make -C "$ROOT" build-local
	if [[ ! -d "$APP_BUNDLE" ]]; then
		echo "error: expected app bundle at $APP_BUNDLE" >&2
		exit 1
	fi
	echo "==> open $APP_BUNDLE"
	open "$APP_BUNDLE"
}

main() {
	case "${1:-dev}" in
	dev)
		cmd_dev
		;;
	bundle)
		cmd_bundle
		;;
	-h | --help | help)
		usage
		;;
	*)
		echo "unknown command: $1" >&2
		usage >&2
		exit 1
		;;
	esac
}

main "$@"
