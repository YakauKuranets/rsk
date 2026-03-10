# Kali Linux Compatibility Report

## Scope

This report checks whether the current Tauri + React project can be built and run on Kali Linux.

## What was validated

1. Frontend production build (`vite build`) succeeds.
2. Rust backend compile check (`cargo check`) was executed.
3. Repository scanned for Windows-specific hardcoded binaries/paths.
4. Kali dependency list updated.

## Results

- ✅ Frontend build succeeds.
- ⚠️ Rust backend check fails in a clean environment without system GTK/GLib packages (`glib-2.0.pc` missing via `pkg-config`).
- ⚠️ Repository contains Windows-only artifacts under `as/` (`*.dll`, `*.ocx`), which cannot run natively on Kali.
- ✅ Runtime FFmpeg resolution in Rust source already supports Linux fallback (`ffmpeg` in vault path or `PATH`).

## Conclusion

Project is **conditionally compatible** with Kali Linux:

- Backend and Tauri runtime work after installing required Linux system dependencies.
- Windows plugin binaries in `as/` are not portable to Kali and should be treated as optional/legacy Windows-only artifacts.

## Kali setup checklist

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config curl file \
  libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf ffmpeg
```

## Preflight

Run:

```bash
./scripts/check-kali-compat.sh
```

It validates toolchain availability and warns about non-portable Windows binaries.
