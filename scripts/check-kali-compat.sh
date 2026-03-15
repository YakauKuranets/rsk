#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

ok() { echo "[OK] $*"; }
warn() { echo "[WARN] $*"; }
fail() { echo "[FAIL] $*"; }

if [[ -f /etc/os-release ]]; then
  . /etc/os-release
  if [[ "${ID:-}" == "kali" || "${ID_LIKE:-}" == *"debian"* ]]; then
    ok "Detected distro: ${PRETTY_NAME:-unknown}"
  else
    warn "Detected distro: ${PRETTY_NAME:-unknown}. Script is tailored for Kali/Debian-based systems."
  fi
else
  warn "/etc/os-release not found; cannot determine distro."
fi

missing=0
check_cmd() {
  if command -v "$1" >/dev/null 2>&1; then
    ok "Command '$1' found"
  else
    fail "Command '$1' is missing"
    missing=1
  fi
}

for cmd in node npm cargo rustc pkg-config ffmpeg; do
  check_cmd "$cmd"
done

if [[ -f "$ROOT_DIR/src-tauri/Cargo.toml" ]]; then
  ok "Rust backend detected: src-tauri/Cargo.toml"
else
  fail "Rust backend manifest not found"
  missing=1
fi

if [[ -f "$ROOT_DIR/package.json" ]]; then
  ok "Frontend manifest detected: package.json"
else
  fail "Frontend manifest not found"
  missing=1
fi

if [[ -d "$ROOT_DIR/as" ]]; then
  if find "$ROOT_DIR/as" -maxdepth 1 \( -name '*.dll' -o -name '*.ocx' \) | grep -q .; then
    warn "Windows-only binary artifacts found in ./as (DLL/OCX). They are not runnable natively on Kali Linux."
  fi
fi

cat <<'PKG'

[INFO] Recommended Kali packages:
  sudo apt update
  sudo apt install -y \
    build-essential pkg-config curl file \
    libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev librsvg2-dev patchelf ffmpeg
PKG

if [[ "$missing" -eq 1 ]]; then
  echo
  fail "Environment is NOT ready for full Kali compatibility checks."
  exit 1
fi

echo
ok "Base Kali compatibility prerequisites look good."
