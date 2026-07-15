## Why

WezTerm accepts the generated iTerm2 card images but renders them relative to whichever cursor position is current when Ratatui flushes the protocol data. Because the application centers each image only after encoding, selected-card and discard previews can drift outside their panels or overwrite unrelated UI content even though their sizes are correct.

## What Changes

- Make terminal-image sizing a separate graphics-runtime operation so the UI can decide the final centered rectangle before requesting protocol data.
- Give local, non-tmux WezTerm iTerm2 output an application-controlled placement lifecycle outside Ratatui's cell buffer: clear changed old rectangles before the UI draw and emit changed images at absolute coordinates afterward.
- Key each independent preview placement by card and final rectangle, emit nothing for unchanged frames, and fail atomically to text if either slot cannot be encoded safely.
- Preserve the existing behavior for tmux, ordinary iTerm2 terminals, Sixel, Kitty, and text mode.
- Store an unmodified `ratatui-image` v11.0.6 source snapshot under `external/ratatui-image/` as an inactive emergency fallback and document its provenance and activation procedure.

## Capabilities

### New Capabilities

- `wezterm-card-placement`: Defines safe, panel-relative card-image placement for local WezTerm and the compatibility behavior for every other graphics backend.

### Modified Capabilities

None.

## Impact

The change affects internal rendering boundaries in `src/graphics.rs` and `src/ui.rs`, preview-cache invalidation, graphics unit tests, UI layout tests, and developer documentation. It adds a source-only fallback directory but does not change the crates.io dependency in `Cargo.toml`, the CLI, settings, game rules, card data, or other terminal protocols.
