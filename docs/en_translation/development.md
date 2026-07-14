# Project Structure and Operation

This article is intended for developers who want to run, debug, or participate in the development of UNO Star Carnival from source code. The game is a completely offline Rust terminal application, using Ratatui for the interface and Crossterm for cross-platform terminal input and output.

## Environment Requirements

- Windows, macOS, or Linux

- Rust 1.91 or later (recommended to install via [rustup](https://rustup.rs/))

- A terminal window with at least `70 × 22` characters

The program does not depend on external services and does not require database or environment variable configuration. Windows Terminal, including WSL, defaults to `Graphics (Beta)`; WezTerm, other Windows terminals, Linux, and macOS default to colored text cards. Users can explicitly enable beta graphics in any local terminal; images appear only at `70 × 26` or larger with a supported protocol and otherwise safely fall back to text.

## Project Structure

```text uno_laptop_client/

├── .github/workflows/ # GitHub Actions and cargo-dist release workflow

├── docs/ # Development notes, manual test checklists, and demo resources

├── external/debug/ # Debugging aids for cargo-dist/axoupdater

├── openspec/ # OpenSpec design, specifications, and task logs for feature changes

├── src/

│ ├── main.rs # Program entry point, event loop, and terminal state recovery

│ ├── app.rs # Page state, input processing, and local game flow

│ ├── core.rs # Cards, decks, rules, turn states, and game events

│ ├── ai.rs # Local AI decision-making at different difficulty levels

│ ├── ui.rs # Ratatui layout, components, and overlay rendering

│ ├── graphics.rs # Terminal image protocol detection, degradation, and preview caching

│ ├── card_art.rs # Generate language-independent UNO card bitmaps from code

│ └── i18n.rs # English and Simplified Chinese interface text

├── Cargo.toml # Rust package information, dependencies, and build configuration

├── Cargo.lock # Locked dependency versions

├── dist-workspace.toml # Cargo-dist installation package and release target configuration

└── README.md # Project introduction and installation instructions for released versions

```

The main call relationships are as follows:

```text
main (terminal initialization and event loop)

├── app (application state and input) ──> core (rules)

│ └──> ai (computer player)

└── ui (interface rendering) ────────> graphics ──> card_art

└──> i18n

```

## Running from Source Code

Execute in the repository root directory:

```console
cargo run
```

On the first run, Cargo will download and compile dependencies. After compilation, it will directly enter the settings page, where you can choose player name, number of AIs, difficulty, deck, language, and graphics mode.

To run with an optimized release configuration:

```console
cargo run --release
```

To view command-line help or the current build information without entering the terminal interface:

```console
cargo run -- --help
cargo run -- --version
```

The `--` separator passes the following argument to `uno` instead of Cargo. `-v` and `--version` are equivalent; the output contains both the Cargo package version and the 12-character Git commit for the build. The program does not accept other positional arguments; all game options are configured in the TUI settings page.

After installing with the shell or PowerShell script from the README, run `uno --uninstall` to review the managed paths and enter `y` or `yes` to confirm. `uno --uninstall -y` and `uno --uninstall --yes` skip the prompt. UNO removes `uno`, `uno-update`, and the receipt only when the cargo-dist receipt matches the running executable. Source, Cargo, package-manager, and manually copied builds are refused and must be removed through their original installation method. Uninstalling does not modify the shared `CARGO_HOME/bin`, shell configuration, or Windows PATH registry entry.

## Building and Running Binaries

Debug Build:

```console
cargo build
```
The generated program is located in:

- Windows: `target\debug\uno.exe`

- macOS / Linux: `target/debug/uno`

Release Build:

```console
cargo build --release
```
The corresponding program is located in the `target/release/` directory.

After building, you can query its version from any directory, for example:

```console
target/release/uno --version
```

The version and commit are embedded in the executable at compile time. A release installed with the script in the README can likewise run `uno --version` without reading `Cargo.toml`, `Cargo.lock`, or `.git` at runtime.

## Development Checks

It is recommended to execute the following before submitting changes:

```console
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Modifications involving terminal rendering, keyboard interaction, or image protocols should also be verified in the target terminal according to the [manual test checklist](manual-test.md).

## Running Notes

- When the terminal is smaller than `70 × 22`, the program will only display a prompt to resize the window.

- Graphical display requires a terminal of at least `70 × 26`; a full text interface will still work if this requirement is not met.

- Setup offers `Text` and `Graphics (Beta)`. Only Windows Terminal, including WSL, defaults to beta graphics; WezTerm and every other terminal default to Text.

- Any local terminal can opt into `Graphics (Beta)` to try a detected iTerm2, Sixel, or Kitty backend. Selecting `Text` disables image output.

- SSH sessions will automatically use text display to avoid image protocol escape sequences interfering with the remote terminal.

- Pressing `Ctrl+C`, exiting normally, or experiencing a panic will attempt to restore the terminal's original mode, cursor, and alternate screen state.

If you only want to install and run the released version, please use the installation command in [README](../README.md#quick-start).
