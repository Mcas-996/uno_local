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
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Mcas-996/uno_online/releases/latest/download/uno-installer.sh | sh
```

### Windows PowerShell

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/Mcas-996/uno_online/releases/latest/download/uno-installer.ps1 | iex"
```

After installation, start the game with:

```console
uno
```

## Updating

Installations created by the shell or PowerShell installer include an updater. Run:

```console
uno-update
```

The updater checks GitHub Releases and installs a newer version when one is available.

## Uninstalling

Installations created by the shell or PowerShell installer can be removed with:

```console
uno --uninstall
```

The command shows the managed files and requires `y` or `yes` before removing them. To skip the prompt, run `uno --uninstall -y` or `uno --uninstall --yes`.

Only a matching cargo-dist installation is removed. Package-manager, Cargo, development, and manually copied builds are refused so they can be removed by the tool that installed them. Uninstalling removes `uno`, `uno-update`, and the install receipt; it preserves the shared Cargo bin directory and PATH configuration.

## Notice

Release artifacts and installers are built with [cargo-dist](https://github.com/axodotdev/cargo-dist). The `uno-update` command is provided by [axoupdater](https://github.com/axodotdev/axoupdater).

## License

This project is licensed under the [GNU Affero General Public License v3.0 only](LICENSE).

## For developer
This README.md is for quick start & acknowledgements, for docs, please go to [developement.md](docs/development.md), if you are curious why is there an extra repository in [here](external/debug), read [this](docs/extra_info.md).

## Issue

If you found any issue, feel free to report it in [issue](https://github.com/Mcas-996/uno_local/issues)
