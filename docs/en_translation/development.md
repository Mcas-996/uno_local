# Project Structure and Operation

UNO Star Carnival is a fully offline Rust terminal application. A single `uno` binary selects one of two frontends at runtime: a universal Crossterm frontend and a WezTerm-specific Termwiz frontend. Both share application state, rules, input semantics, and the layout model.

## Requirements and frontend selection

- Windows, macOS, or Linux
- Rust 1.91 or later
- A terminal of at least `70 × 26` cells

Frontend precedence is deterministic:

1. SSH and tmux sessions force the universal text mode.
2. Local WezTerm sessions use Termwiz and default to graphics; Text remains selectable in setup.
3. Other local terminals use the universal frontend. It defaults to graphics only after confirming Sixel support and valid cell-pixel dimensions; otherwise it uses Text.

## Source layout

```text
src/
├── main.rs          # entry point, frontend dispatch, panic restoration
├── environment.rs   # SSH, tmux, and WezTerm classification
├── frontend.rs      # frontend-neutral input, viewport, and display types
├── app.rs           # application state and local game flow
├── view.rs          # shared semantic view and navigation
├── screen.rs        # custom cell buffer, layout, and image slots
├── universal.rs     # Crossterm text rendering and direct Sixel output
├── termwez.rs       # Termwiz Surface, input, and WezTerm images
├── core.rs / ai.rs  # game rules and AI
├── card_art.rs      # language-independent card bitmaps
└── i18n.rs          # English and Simplified Chinese strings
```

The project no longer depends on Ratatui or `ratatui-image` and contains no application-owned Kitty/iTerm2 escape-sequence renderer. The universal frontend encodes Sixel directly with `icy_sixel`. The WezTerm frontend supplies PNG `EncodedFile` data to Termwiz `Change::Image`.

## Run and build

```console
cargo run
cargo run --release
cargo run -- --help
cargo run -- --version
```

`-v` is equivalent to `--version`. Game options are configured in the setup screen.

Debug binaries are written to `target\debug\uno.exe` on Windows or `target/debug/uno` on macOS/Linux. Release binaries are written to `target/release/`.

An installer-managed copy can be removed with `uno --uninstall` or without prompting with `uno --uninstall -y`. UNO only removes files when the cargo-dist receipt matches the running executable.

## Development checks

```console
cargo fmt --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Rendering, input, and image changes must also follow the [manual terminal matrix](../manual-test.md).

## Rendering and fallback behavior

- The universal frontend sends Primary DA, `CSI 16 t`, and DSR once at startup and parses the bounded response. Graphics require both a Sixel declaration and valid cell-pixel dimensions.
- Sixel output is cached by card, target cell size, and cell-pixel size. Moving, resizing, overlays, and mode changes clear obsolete image regions.
- The Termwiz frontend caches PNG data and renders images through Surface cells. PNG generation failure keeps Termwiz running in Text mode.
- A Termwiz initialization or terminal I/O failure preserves the current `App` and falls back to universal Text.
- Help, color picker, result, and quit overlays suppress images. Below `70 × 26`, only the resize message is rendered.
- Normal exit, `Ctrl+C`, and panic attempt to restore cooked mode, the cursor, and the primary screen.
