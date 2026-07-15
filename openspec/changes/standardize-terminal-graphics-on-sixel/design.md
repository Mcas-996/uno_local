## Context

The application currently exposes iTerm2 IIP, Sixel, Kitty, and Text results through `GraphicsBackend`. WezTerm is deliberately forced to IIP and then receives an application-owned absolute-cursor wrapper because upstream `ratatui-image 11.0.6` blacklists WezTerm Sixel. Other terminals accept any detected image protocol. This creates terminal-specific safety validation, causes position changes to invalidate protocol caches, and requires an inactive upstream source snapshot solely as an emergency IIP patch point.

The product policy already defaults only Windows Terminal to Graphics Beta. WezTerm and other terminals default to Text but can opt in, so reducing Graphics Beta to one protocol does not change the default experience outside Windows Terminal. The crates.io dependency remains the protocol implementation boundary and its unused internal Kitty/IIP modules are not application capabilities.

## Goals / Non-Goals

**Goals:**

- Make Sixel the only image backend selectable by the application.
- Produce deterministic terminal mapping and fallback behavior, including no capability query over SSH.
- Remove application-owned IIP positioning and validation code.
- Avoid Sixel re-encoding when only a panel's screen position changes.
- Preserve stable Text fallback after an encoding error and preserve selected/discard cache independence.
- Align source, tests, UI labels, active documentation, and repository content with the supported capability.

**Non-Goals:**

- Reimplement Sixel escape sequences or fork `ratatui-image`.
- Remove unused protocol modules from transitive dependency source or metadata.
- Add a protocol selector or change persisted settings.
- Change the terminal-specific default Graphics Beta selection policy.
- Rewrite completed historical OpenSpec artifacts.

## Decisions

### Resolve every graphics request to Sixel or Text

`GraphicsBackend` will contain only `Sixel` and `Text(FallbackReason)`. SSH resolves before picker creation/query. Every local terminal enables graphics only when the picker actually reports Sixel. Kitty, IIP, Halfblocks, query failure, and no result map to `Text(Unsupported)`.

WezTerm remains Text because `ratatui-image 11.0.6` deliberately blacklists its Sixel path: the upstream compatibility matrix identifies only the removed alternative protocol as bug-free, and real-terminal validation on WezTerm `20240203-110809-5046fc22` produced blank image panels when the blacklist was bypassed. Respecting the picker result was chosen over forcing Sixel because a usable Text interface is preferable to claiming a graphical backend that does not render.

This retains explicit terminal identification only where it changes policy. Retaining multi-protocol variants was rejected because it preserves support obligations and divergent rendering behavior.

### Keep the crates.io protocol boundary

The runtime will continue to ask `ratatui-image 11.0.6` to create and render Sixel protocols. The application will not inspect or construct protocol-specific escape data. The inactive `external/ratatui-image/` snapshot will be deleted because its only intended use was an IIP placement emergency patch.

Writing Sixel escape sequences locally was rejected because encoding, cleanup, tmux passthrough, and Ratatui integration remain library responsibilities.

### Separate geometry from encoded protocol identity

The rendering pipeline remains `fit_size` → UI-centered `Rect` → protocol rendering. Cache identity changes from `(Card, Rect)` to `(Card, Size)`, because Sixel protocol data does not contain the UI screen origin. Selected and discard slots remain separate so equal cards in distinct panels do not share mutable protocol state.

Caching by `Rect` was rejected because panel movement triggers needless image encoding. Sharing a global cache was rejected because protocol render state is mutable and the two preview roles require independent invalidation.

### Make encoding failure terminal for the runtime session

If protocol creation fails, the runtime sets `Text(Encoding)`, drops the picker, clears both cache slots, and does not retry during that process lifetime. This avoids repeated expensive failures and partial graphics output while keeping the full text UI available.

### Express supersession in the new change

The new capability spec is the current contract and explicitly supersedes the completed change artifacts that allowed Kitty/IIP, required WezTerm IIP positioning, or retained the external snapshot. Those historical files remain unchanged for traceability.

## Risks / Trade-offs

- [Some terminals support Kitty or IIP but not Sixel] → They receive the complete Text interface and the setup status states that graphics are unsupported.
- [WezTerm has preliminary Sixel support but the dependency blacklists it] → Preserve the complete Text interface and do not override the dependency's compatibility decision; reconsider only after an upstream version documents and detects a reliable WezTerm Sixel path.
- [Sixel terminal placement differs across emulators or tmux] → Continue using `ratatui-image` rendering/cleanup and validate panel containment, resize, overlays, and exit cleanup in the terminal matrix.
- [A future Sixel encoder embeds position] → The cache key assumption would need revisiting when upgrading `ratatui-image`; document `(Card, Size)` as an invariant.

## Migration Plan

1. Add the superseding capability spec and implementation tasks.
2. Simplify backend resolution, protocol creation, cache identity, labels, and tests.
3. Remove the external snapshot and update active documentation.
4. Run formatting, build, unit tests, and warning-clean Clippy checks.
5. Exercise the documented real-terminal matrix before release. Rollback is a normal source revert; no persisted data migration is involved.

## Open Questions

None. The capability boundary, dependency version, default selection policy, and fallback behavior are fixed by this change.
