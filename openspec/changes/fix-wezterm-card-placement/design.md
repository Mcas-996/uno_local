## Context

`GraphicsRuntime` currently encodes a card for an available `Size`, and `ui.rs` then centers the protocol's returned size inside the preview panel. The iTerm2 protocol stores one escape-sequence string in the first Ratatui cell and marks the remaining image cells as skipped. In local WezTerm, that string clears and draws relative to the terminal cursor that happens to be current when the backend flushes the buffer; it does not know the centered rectangle chosen afterward. The result is correctly scaled image data anchored at the wrong location.

The fix must stay in the application because other protocol implementations and tmux passthrough are already handled correctly by `ratatui-image`. The crates.io dependency remains the default, while an exact v11.0.6 source snapshot provides a reviewed emergency patch point if a future upstream interface makes safe application wrapping impossible.

## Goals / Non-Goals

**Goals:**

- Give UI layout ownership of the final image rectangle while graphics retains image fitting and protocol encoding ownership.
- Establish an explicit cursor contract for local, non-tmux WezTerm iTerm2 output.
- Invalidate cached protocol data when either the card, origin, or dimensions change.
- Fail once and remain in text mode if WezTerm iTerm2 data cannot be recognized and wrapped safely.
- Preserve upstream behavior for tmux, ordinary iTerm2 terminals, Sixel, Kitty, and text rendering.
- Preserve a traceable, inactive `ratatui-image` v11.0.6 source fallback.

**Non-Goals:**

- Fix independent placement defects in Windows Terminal, remote sessions, or other terminals.
- Change card art, scaling policy, panel height, CLI options, settings, or game behavior.
- Enable or modify the vendored fallback in the normal build.
- Change `docs/extra_info.md`.

## Decisions

### Separate fitting from placement and encoding

`GraphicsRuntime::fit_size` will use the cached card art, the picker's detected font-cell size, and `Resize::Fit(None)` to return the protocol size for the panel's available cells without encoding. `ui.rs` will center that size and pass the complete final `Rect` to `GraphicsRuntime::protocol`. This preserves the existing aspect ratio and keeps the architectural boundary clear: graphics determines protocol dimensions; UI determines screen coordinates.

Encoding first and rewriting the rectangle afterward was rejected because it recreates the current missing-position problem. Moving centering into graphics was rejected because graphics would then own page layout that belongs to the UI.

### Make protocol caches rectangle-aware

Each selected/discard slot will retain its independent cache, but `ProtocolKey` will contain `(Card, Rect)` instead of `(Card, Size)`. The rectangle dimensions feed upstream encoding and the origin feeds the WezTerm cursor wrapper. Identical rectangles reuse the encoding; any origin or dimension change rebuilds it. Separate slots remain necessary even when both display the same card.

### Wrap only recognizable local WezTerm iTerm2 data

The runtime will retain terminal-environment information from detection. After upstream encoding, it will alter data only when all of these are true: the session is WezTerm, the protocol variant is iTerm2, and the upstream protocol says it is not using tmux passthrough.

The wrapper will verify the exact upstream clear-area prefix for the encoded dimensions and the expected iTerm2 image introducer. It then prepends a one-based absolute cursor position for the rectangle's top-left and appends a one-based absolute position for the next cell Ratatui accounts for after the string-bearing cell. Upstream clearing and image bytes remain unchanged.

Saving/restoring the terminal cursor was rejected because Ratatui expects output to advance by one cell, not return to the pre-write cursor. Relative movement was rejected because the bug is caused by an unreliable starting cursor.

### Treat unsafe wrapping as an encoding failure

If a WezTerm protocol has an unexpected variant, tmux state, clear sequence, or iTerm2 framing that requires wrapping but cannot be verified, the runtime will switch to `Text(Encoding)`, drop the picker, and clear both preview caches. It will not emit unanchored image data or retry each frame. This reuses the existing stable-degradation behavior.

### Keep an inactive upstream fallback snapshot

The crates.io v11.0.6 package source, corresponding to upstream commit `a813cde9d83139bc87f64fe167abeb690b74019a`, will be copied to `external/ratatui-image/` without `.git`, `target`, or Cargo unpack markers. Licenses, Cargo manifests/lockfile, source, upstream tests/snapshots, examples, benches, and documentation remain intact. `Cargo.toml` continues to resolve the crates.io release. Developer documentation will describe provenance, maintenance rules, and the emergency `[patch.crates-io]` stanza.

## Risks / Trade-offs

- [The wrapper relies on the public iTerm2 data layout of v11.0.6] → Verify the clear prefix and image introducer before modifying data; fail closed to text if upstream changes.
- [Absolute cursor positions use CSI coordinates with u16 layout values] → Convert from Ratatui's zero-based coordinates with saturating arithmetic and test the exact one-based sequences.
- [A position-only change now re-encodes PNG/base64 data] → This is required because the cached output embeds the origin; normal 50 ms redraws at an unchanged rectangle still reuse the cache.
- [A full source snapshot increases repository size and maintenance surface] → Keep it inactive and unmodified, record its checksum/commit, and update it only as an intentional dependency-maintenance task.
- [Automated tests cannot prove terminal emulator behavior] → Combine unit coverage of bytes and rectangles with the documented WezTerm, Windows Terminal, and text-mode manual matrix.

## Migration Plan

1. Land the application-level API, cursor wrapper, cache changes, tests, source snapshot, and documentation together.
2. Keep the normal crates.io dependency and verify all automated checks plus the WezTerm manual matrix.
3. If the application wrapper later becomes impossible or fails acceptance, review and modify only the snapshot, add the documented `[patch.crates-io]` stanza, and rerun the full matrix.
4. Roll back by removing the application wrapper and snapshot; no persisted user data or configuration migration is involved.

## Open Questions

None for this change. Other terminal-specific placement defects require separate changes.
