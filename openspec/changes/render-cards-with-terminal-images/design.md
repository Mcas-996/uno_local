## Context

The application redraws a Ratatui/Crossterm alternate-screen UI every 50 ms and currently represents every card as styled text. The enhancement crosses terminal initialization, setup state, responsive layout, localization, image generation, and cleanup. It must remain usable on the existing 70x22 minimum screen, across native Windows shells and WSL, and without sending image traffic through SSH sessions.

## Goals / Non-Goals

**Goals:**
- Render generated card art for the selected human card and discard-top card on sufficiently large local terminals.
- Use WezTerm's stable iTerm2 path and Windows Terminal's Sixel path, with Kitty and other supported protocols available on non-Windows terminals.
- Make backend selection visible and deterministic, and make every failure fall back to the existing text card renderer.
- Avoid redundant image encoding, flicker, overlay bleed, and terminal residue.

**Non-Goals:**
- Image rendering for every hand card, opponent card backs, the draw pile, setup decorations, or animation.
- Sending terminal images through SSH, supporting halfblock image approximations, or forcing an experimental protocol.
- Changing rules, AI, commands, or the existing 70x22 minimum usable size.

## Decisions

### Use ratatui-image as the protocol boundary

Upgrade to the Ratatui/Crossterm versions required by `ratatui-image 11.0.6`, add `image 0.25.x`, and disable Chafa default features so release binaries do not require an external Chafa library. The library owns protocol queries, cell-to-pixel sizing, encoding, tmux awareness, and Ratatui integration. Hand-written Kitty/Sixel escape sequences were rejected because they would duplicate placement, chunking, query, and cleanup logic.

### Resolve a restricted backend once per terminal session

Introduce an internal `GraphicsChoice` (`Auto`, `Text`), `GraphicsBackend` (`Iterm2`, `Sixel`, `Kitty`, `Text`), and text fallback reason. Detect SSH from a non-empty `SSH_CONNECTION`, `SSH_CLIENT`, or `SSH_TTY`; do not use `SSH_AUTH_SOCK`, which can identify agent access without an SSH login. SSH resolves directly to `Text(Ssh)` and skips all graphics queries.

For local Auto mode, enter the alternate screen and call `Picker::from_query_stdio()` before starting event reads. Resolve in this order:
1. WezTerm (`WEZTERM_EXECUTABLE` or `TERM_PROGRAM` containing `WezTerm`) accepts only iTerm2; this check precedes `WT_SESSION` to tolerate inherited environment variables.
2. Windows Terminal (`WT_SESSION`) accepts only Sixel, covering native shells and local WSL.
3. Other local terminals accept a successfully detected Kitty, Sixel, or iTerm2 backend.
4. Query failure, halfblocks, an unexpected backend, or later encoding failure resolves to Text.

Selecting Text in setup suppresses image rendering without re-querying the terminal. The setup row displays both the choice and the effective result, such as `Auto (iTerm2)`, `Auto (Sixel)`, `Auto (Text: SSH)`, or `Text (manual)`. The choice follows existing setup state and resets to Auto on each launch; no configuration file is introduced.

### Generate universal card art in memory

Generate an RGBA card face for every `Card` using pixel drawing primitives and a small built-in ASCII bitmap glyph set. Use colored rounded bodies, number/action symbols, four-color Wild artwork, and explicit +2/+4/+8/+16 markings. Raster art is language-neutral; localized text card names remain in the surrounding panel title for accessibility and unambiguous identification. This avoids dozens of binary assets, font licensing, and missing Holiday artwork.

### Keep graphics state outside game/application rules

The terminal runner owns a mutable graphics runtime passed into UI rendering. It caches generated base art by `Card` and maintains separate selected-card and discard-card protocol slots keyed by card, backend, and target cell size. A slot is re-encoded only when one of those keys changes; unchanged 50 ms redraws reuse the encoded protocol.

`App` owns only the user's Auto/Text setup choice. It does not own image buffers, terminal queries, or protocol placements, keeping reducer and game tests independent from real terminal I/O.

### Use responsive image and text layouts

Keep `MIN_WIDTH=70` and `MIN_HEIGHT=22`. At less than 70x26, render the current colored number/action text and no image widgets. At 70x26 or larger, allocate a nine-row table area with side-by-side selected-card and discard-top panels, fit one image inside each panel, and retain localized titles plus active-color text. If the human hand is empty, the selected panel shows a text placeholder.

While help, quit confirmation, result, or wild-color overlays are visible, invalidate both image placements and render the text table beneath the overlay. Re-entering the unobscured game rebuilds the appropriate placements. Resize across the threshold, new matches, backend changes, and shutdown similarly invalidate slots and force a clean redraw.

## Risks / Trade-offs

- [Windows Terminal Sixel behavior differs across releases] -> Accept Sixel only after a successful query, fall back to Text on any construction failure, and manually test version 1.22 or newer.
- [WezTerm exposes partial Kitty support] -> Identify WezTerm explicitly and accept only its stable iTerm2 inline-image path.
- [Environment variables can be inherited or manually forged] -> Give SSH highest priority, WezTerm precedence over `WT_SESSION`, and show the resolved backend/reason in setup.
- [Image encoding can block interaction] -> Generate small rasters, keep only two protocol slots, and encode only when card or size changes.
- [Sixel or placement images can bleed through overlays or survive resize] -> Suspend graphics for overlays, invalidate on every layout transition, redraw cleared cells, and verify cleanup in native Windows and WSL manual tests.
- [Dependency upgrades can change rendering snapshots] -> Update Ratatui tests in the same change and retain a fully testable Text renderer.

## Migration Plan

Add the dependencies and backend abstraction first, preserve Text as the default failure path, then add generated art and the large-screen layout. Update automated and manual tests before enabling Auto as the setup default. Rollback consists of removing the graphics runtime and dependencies; no persisted data or protocol migration is required.

## Open Questions

None.
