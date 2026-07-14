<div align="center">

# ✨ UNO Star Carnival ✨

🌟 **Standard UNO. Holiday chaos. Fully offline.** 🌟

[![Rust](https://img.shields.io/badge/Rust-1.91%2B-f74c00?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-27d8e8)](#quick-start)
[![License: AGPL v3](https://img.shields.io/badge/license-AGPL--3.0--only-f7f73b)](LICENSE)
[![Release](https://github.com/Mcas-996/uno_online/actions/workflows/release.yml/badge.svg)](https://github.com/Mcas-996/uno_online/actions/workflows/release.yml)



https://github.com/user-attachments/assets/37a3b0e1-1527-4067-b81b-d49c915f5e90






## Quick Start

### macOS / Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Mcas-996/uno_online/releases/download/v0.5.3/uno-installer.sh | sh
```

### Windows PowerShell

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/Mcas-996/uno_online/releases/download/v0.5.3/uno-installer.ps1 | iex"
```

After installation, start the game with:

```console
uno
```

## Terminal Card Graphics

Card previews are enabled automatically on local terminals with a supported image protocol when the window is at least 70 × 26 cells. Windows WezTerm uses iTerm2 inline images, while Windows Terminal 1.22 or newer uses Sixel. Smaller windows, unsupported terminals, explicit Text mode, and every detected SSH session retain the fully playable colored text cards.

The setup screen shows the resolved graphics backend. Choose `Graphics: Text` there to disable image output for the current run.

## Updating

Installations created by the shell or PowerShell installer include an updater. Run:

```console
uno-update
```

The updater checks GitHub Releases and installs a newer version when one is available.

## Distribution

Release artifacts and installers are built with [cargo-dist](https://github.com/axodotdev/cargo-dist). The `uno-update` command is provided by [axoupdater](https://github.com/axodotdev/axoupdater).

## License

This project is licensed under the [GNU Affero General Public License v3.0 only](LICENSE).

## Issue

If you found any issue, feel free to report it in [issue](https://github.com/Mcas-996/uno_local/issues)
