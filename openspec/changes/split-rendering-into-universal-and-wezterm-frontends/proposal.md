## Why

The current Ratatui plus terminal-image stack has accumulated mutually incompatible Sixel, iTerm2, and application-owned Kitty placement paths. Replacing it with two deliberately bounded frontends keeps the complete game usable everywhere while giving local WezTerm a renderer designed around WezTerm's own terminal model.

## What Changes

- **BREAKING** Remove Ratatui, ratatui-image, and the application-owned Kitty/iTerm2 placement runtime.
- Ship one `uno` binary that dispatches to a Crossterm universal frontend or a Termwiz WezTerm frontend at runtime.
- Keep SSH and tmux on the universal text path without graphics queries.
- Give the universal frontend explicit Text/Sixel rendering with direct capability detection and pure-Rust Sixel encoding.
- Give local WezTerm a Termwiz `Surface` frontend with Termwiz-owned iTerm2 image rendering and text fallback.
- Move terminal-specific key events, layout, and graphics state out of the shared application/game state.
- Preserve the existing CLI, installer, updater, uninstall behavior, languages, game rules, and minimum terminal size.

## Capabilities

### New Capabilities

- `dual-terminal-frontends`: Defines runtime frontend selection, shared behavior, universal Text/Sixel rendering, Termwiz WezTerm rendering, fallback, and cleanup requirements.

### Modified Capabilities

None. This capability supersedes the historical graphics-selection, Sixel-only, and WezTerm-placement delta requirements that were never synchronized into main specs.

## Impact

The change restructures the application/input boundary and replaces `src/ui.rs` and `src/graphics.rs` with shared view data plus two frontend implementations. Dependencies change from Ratatui/ratatui-image to direct Crossterm, Termwiz, icy_sixel, and PNG encoding. Rendering tests, terminal-environment tests, active documentation, and the manual terminal matrix are replaced, while cargo-dist continues to publish one package and one executable.
