# Tauri + React

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Project documentation

- [Архитектура системы (RU)](./ARCHITECTURE_RU.md)


## Важно по статусу реализации

- Документация описывает архитектуру и текущее состояние, но не заменяет трекинг задач и проверку по исходному коду.
- Детали: [Архитектура системы (RU)](./ARCHITECTURE_RU.md).

## Kali Linux

- Backend now resolves `Nemesis Vault` path dynamically on Linux: `~/.nemesis_vault/recon_db`.
- FFmpeg launch now supports Linux (`ffmpeg` from vault path or from `PATH`) instead of hardcoded `ffmpeg.exe`.
- For Kali install build/runtime dependencies before `npm run tauri dev`: `sudo apt install -y build-essential pkg-config libglib2.0-dev ffmpeg libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf`.


- Quick preflight for Kali: `./scripts/check-kali-compat.sh`.
