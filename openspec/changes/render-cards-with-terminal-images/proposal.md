## Why

The current TUI identifies cards through compact colored text only, leaving capable local terminals unable to present recognizable card artwork. Add an optional image-enhanced presentation while preserving the existing text experience for small, remote, or unsupported terminals.

## What Changes

- Generate card faces in code and render the selected human card and discard-top card as terminal images when the screen is large enough.
- Automatically select the stable image protocol for supported terminals, including iTerm2 inline images for WezTerm and Sixel for Windows Terminal.
- Detect SSH sessions before probing terminal graphics and force the text renderer for remote play.
- Add an Auto/Text graphics choice and report the resolved backend on the setup screen.
- Retain the current colored number/action labels as the fallback for small terminals, unsupported protocols, failed capability queries, overlays, and explicit Text mode.

## Capabilities

### New Capabilities
- `terminal-card-images`: Responsive card-image generation, terminal/backend selection, SSH-safe fallback, graphics settings, and image lifecycle behavior.

### Modified Capabilities

None.

## Impact

- Affects the terminal lifecycle and rendering paths in `src/main.rs`, `src/app.rs`, `src/ui.rs`, plus localization and manual terminal tests.
- Adds image generation and Ratatui image-protocol dependencies and requires compatible Ratatui/Crossterm versions.
- Does not change game rules, AI behavior, saved data, networking, or command semantics.
