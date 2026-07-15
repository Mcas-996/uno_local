## Why

Supporting multiple terminal image protocols has left the application with terminal-specific branches, unsafe placement workarounds, and documentation that overstates the reliable graphics surface. Standardizing Graphics Beta on reliably detected Sixel gives supported terminals one predictable image path while preserving the complete text UI everywhere else.

## What Changes

- **BREAKING** Remove Kitty and iTerm2 Inline Images Protocol (IIP) from the application's supported graphics backends, labels, tests, and active documentation.
- Make Graphics Beta mean Sixel exclusively: accept only detected Sixel and map every other result to Text. Respect `ratatui-image`'s WezTerm Sixel blacklist so that WezTerm degrades to Text instead of emitting a known-buggy image path.
- Keep SSH on Text without querying graphics capabilities and keep the existing default choice policy: Windows Terminal defaults to Graphics Beta; WezTerm and other terminals default to Text.
- Remove the WezTerm IIP absolute-position wrapper and related validation, compatibility branches, and tests.
- Cache encoded Sixel protocols by card and fitted size so moving a panel does not re-encode the image; keep independent selected and discard caches.
- Preserve stable runtime fallback to Text after encoding failure, clearing the picker and both caches.
- Remove the inactive `external/ratatui-image/` emergency snapshot while continuing to use crates.io `ratatui-image 11.0.6` for Sixel encoding and Ratatui integration.
- Update active development, graphics architecture, translation, and manual-test documentation for the Sixel/Text capability model.
- Supersede historical requirements that select Kitty/IIP, require WezTerm IIP or absolute-coordinate wrapping, or require the external emergency snapshot; historical artifacts remain unchanged.

## Capabilities

### New Capabilities

- `sixel-terminal-graphics`: Defines the Sixel-only Graphics Beta selection, rendering, caching, cleanup, fallback, user-interface, and terminal verification behavior that supersedes the earlier multi-protocol terminal graphics requirements.

### Modified Capabilities

None. The repository has no synchronized main specs; this change introduces the current superseding contract while retaining completed historical change artifacts.

## Impact

- Affected code: `src/graphics.rs`, graphics labels in `src/i18n.rs`, and graphics-related UI tests in `src/ui.rs`.
- Affected documentation: `docs/development.md`, `docs/en_translation/development.md`, `docs/ui_graphics_and_card.md`, and `docs/manual-test.md`.
- Affected repository content: `external/ratatui-image/` is deleted; `Cargo.toml` continues to resolve `ratatui-image 11.0.6` from crates.io.
- User-visible impact: unsupported graphics protocols now produce the existing usable Text interface instead of terminal images.
